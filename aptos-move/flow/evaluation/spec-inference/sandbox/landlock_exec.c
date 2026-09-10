// Execute a command with filesystem access restricted to explicit path trees.
#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <linux/landlock.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <sys/stat.h>
#include <unistd.h>

static const __u64 READ_ACCESS = LANDLOCK_ACCESS_FS_EXECUTE |
                                 LANDLOCK_ACCESS_FS_READ_FILE |
                                 LANDLOCK_ACCESS_FS_READ_DIR;
// `LANDLOCK_ACCESS_FS_TRUNCATE` arrived in ABI 3. Left out of the handled mask,
// truncation is ungoverned: a file the ruleset does not make writable can still
// be emptied through `O_TRUNC` or `ftruncate`. Older headers may not define it.
#ifndef LANDLOCK_ACCESS_FS_TRUNCATE
#define LANDLOCK_ACCESS_FS_TRUNCATE (1ULL << 14)
#endif

static const __u64 WRITE_ACCESS = LANDLOCK_ACCESS_FS_WRITE_FILE |
                                  LANDLOCK_ACCESS_FS_REMOVE_DIR |
                                  LANDLOCK_ACCESS_FS_REMOVE_FILE |
                                  LANDLOCK_ACCESS_FS_MAKE_CHAR |
                                  LANDLOCK_ACCESS_FS_MAKE_DIR |
                                  LANDLOCK_ACCESS_FS_MAKE_REG |
                                  LANDLOCK_ACCESS_FS_MAKE_SOCK |
                                  LANDLOCK_ACCESS_FS_MAKE_FIFO |
                                  LANDLOCK_ACCESS_FS_MAKE_BLOCK |
                                  LANDLOCK_ACCESS_FS_MAKE_SYM |
                                  LANDLOCK_ACCESS_FS_REFER;

static void fail(const char *message, const char *path) {
    if (path != NULL) {
        fprintf(stderr, "landlock-exec: %s `%s`: %s\n", message, path,
                strerror(errno));
    } else {
        fprintf(stderr, "landlock-exec: %s: %s\n", message, strerror(errno));
    }
    exit(125);
}

static void add_path_rule(int ruleset_fd, const char *path, __u64 access) {
    int path_fd = open(path, O_PATH | O_CLOEXEC);
    if (path_fd < 0) {
        fail("cannot open allowed path", path);
    }
    struct stat metadata;
    if (fstat(path_fd, &metadata) < 0) {
        close(path_fd);
        fail("cannot inspect allowed path", path);
    }
    if (!S_ISDIR(metadata.st_mode)) {
        access &= LANDLOCK_ACCESS_FS_EXECUTE | LANDLOCK_ACCESS_FS_READ_FILE |
                  LANDLOCK_ACCESS_FS_WRITE_FILE | LANDLOCK_ACCESS_FS_TRUNCATE;
    }
    struct landlock_path_beneath_attr rule = {
        .allowed_access = access,
        .parent_fd = path_fd,
    };
    if (syscall(SYS_landlock_add_rule, ruleset_fd, LANDLOCK_RULE_PATH_BENEATH,
                &rule, 0) < 0) {
        close(path_fd);
        fail("cannot add allowed path", path);
    }
    close(path_fd);
}

int main(int argc, char **argv) {
    int abi = syscall(SYS_landlock_create_ruleset, NULL, 0,
                      LANDLOCK_CREATE_RULESET_VERSION);
    if (abi < 1) {
        fail("Landlock is unavailable", NULL);
    }
    // Truncation is a distinct access right, and one the kernel cannot govern
    // before ABI 3: a read-only path could be emptied -- the pristine baseline
    // or the run record among them -- while the policy reported itself intact.
    // There is no way to close that on such a kernel, so the round refuses to
    // run rather than claim a guarantee it does not have.
    if (abi < 3) {
        fail("Landlock ABI 3 or later is required to govern truncation", NULL);
    }

    // Governing an access the running kernel does not know rejects the ruleset
    // outright. A writable path is granted truncation too, or the agent could
    // not rewrite its own workspace.
    __u64 write_access = WRITE_ACCESS | LANDLOCK_ACCESS_FS_TRUNCATE;
    __u64 handled = READ_ACCESS | write_access;
    struct landlock_ruleset_attr ruleset = {.handled_access_fs = handled};
    int ruleset_fd = syscall(SYS_landlock_create_ruleset, &ruleset,
                             sizeof(ruleset), 0);
    if (ruleset_fd < 0) {
        fail("cannot create ruleset", NULL);
    }

    int index = 1;
    for (; index < argc; index++) {
        if (strcmp(argv[index], "--") == 0) {
            index++;
            break;
        }
        if (index + 1 >= argc) {
            fprintf(stderr, "usage: landlock-exec [--ro PATH|--rw PATH]... -- COMMAND...\n");
            return 2;
        }
        __u64 access;
        if (strcmp(argv[index], "--ro") == 0) {
            access = READ_ACCESS;
        } else if (strcmp(argv[index], "--rw") == 0) {
            access = READ_ACCESS | write_access;
        } else {
            fprintf(stderr, "landlock-exec: unknown option `%s`\n", argv[index]);
            return 2;
        }
        add_path_rule(ruleset_fd, argv[++index], access);
    }
    if (index >= argc) {
        fprintf(stderr, "landlock-exec: command is required\n");
        return 2;
    }
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) {
        fail("cannot set no_new_privs", NULL);
    }
    if (syscall(SYS_landlock_restrict_self, ruleset_fd, 0) < 0) {
        fail("cannot restrict process", NULL);
    }
    close(ruleset_fd);
    execvp(argv[index], &argv[index]);
    fail("cannot execute command", argv[index]);
}
