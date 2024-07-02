use flags::Group;
use xshell::Shell;

mod builder;
mod flags;

fn main() {
    let cmd = flags::Xbuild::from_env_or_exit();

    let sh = Shell::new().unwrap();
    match cmd.subcommand {
        flags::XbuildCmd::Node(node) => node.run(sh),
        flags::XbuildCmd::Tools(tools) => tools.run(sh),
        flags::XbuildCmd::Indexer(indexer) => indexer.run(sh),
        flags::XbuildCmd::Group(group) => group.run(sh),
    };
}
