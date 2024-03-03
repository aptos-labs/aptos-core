#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <system_error>
#include <stdexcept>

#include "fileloader.hpp"

namespace BinFileUtils {

FileLoader::FileLoader(const std::string& fileName)
{
    struct stat sb;

    fd = open(fileName.c_str(), O_RDONLY);
    if (fd == -1)
        throw std::system_error(errno, std::generic_category(), "open");


    if (fstat(fd, &sb) == -1) {          /* To obtain file size */
        close(fd);
        throw std::system_error(errno, std::generic_category(), "fstat");
    }

    size = sb.st_size;

    addr = mmap(NULL, size, PROT_READ, MAP_PRIVATE, fd, 0);
}

FileLoader::~FileLoader()
{
    munmap(addr, size);
    close(fd);
}

} // Namespace
