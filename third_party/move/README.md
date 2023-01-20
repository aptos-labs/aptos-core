
[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE)
[![Discord chat](https://img.shields.io/discord/903339070925721652.svg?logo=discord&style=flat-square)](https://discord.gg/M95qX3KnG8)

![Move logo](assets/color/SVG/Move_Logo_Design_Digital_Final_-01.svg)

# The Move Language

Move is a programming language for writing safe smart contracts originally developed at Facebook to power the Diem blockchain. Move is designed to be a platform-agnostic language to enable common libraries, tooling, and developer communities across diverse blockchains with vastly different data and execution models. Move's ambition is to become the "JavaScript of web3" in terms of ubiquity--when developers want to quickly write safe code involving assets, it should be written in Move.

This repository is the official home of the Move virtual machine, bytecode verifier, compiler, prover, package manager, and book. For Move code examples and papers, check out [awesome-move](https://github.com/MystenLabs/awesome-move).

## Quickstart

### Build the [Docker](https://www.docker.com/community/open-source/) Image for the Command Line Tool

```
docker build -t move/cli -f docker/move-cli/Dockerfile .
```

### Build a Test Project

```
cd ./language/documentation/tutorial/step_1/BasicCoin
docker run -v `pwd`:/project move/cli build
```

Follow the [language/documentation/tutorial](./language/documentation/tutorial/README.md) to set up move for development.

## Community

* Join us on the [Move Discord](https://discord.gg/cPUmhe24Mz).
* Browse code and content from the community at [awesome-move](https://github.com/MystenLabs/awesome-move).

## License

Move is licensed as [Apache 2.0](https://github.com/move-language/move/blob/main/LICENSE).
