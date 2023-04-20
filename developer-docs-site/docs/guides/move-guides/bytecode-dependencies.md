---
title: "Bytecode for Dependencies"
id: "bytecode-dependencies"
---

# Use Bytecode for Dependencies when compiling Move

Aptos enables you to use local bytecode as dependencies when the Move source code for those dependencies are not available locally. To use this feature, you will co-locate the files in directories at the same level and then specify their paths in the corresponding `Move.toml` files.

## Requirements and limitations

Using local bytecode as dependencies requires bytecode files to be downloaded into local, and the actual address for each named address must be known and specified in either `Move.toml` or through `--named-addresses`.

The `aptos move prove` and `aptos move test` commands do not support bytecode as dependencies.

## Recommended structure

We use an example to illustrate the dev flow of using this feature. Suppose we want to compile the package `A` . The package layout is:

```rust
./A
├── Move.toml 
├── sources
  ├ A.move
```

`A.move` is defined below, depending on the modules `Bar`and `Foo`:

```rust
module A::A {
    use B::Bar;
    use C::Foo;
    public fun foo(): u64 {
        Bar::foo() + Foo::bar()
    }
}
```

Suppose the source of `Bar` and `Foo` are not available but the corresponding bytecode `Bar.mv` and `Foo.mv` are available locally. To use them as dependencies, we would:

Specify `Move.toml` for `Bar` and `Foo` . Note that named addresses are already instantiated with the actual address in the bytecode. In our example, the actual address for `C` is already bound to `0x3`.  As a result, `[addresses]`  must be specified `C` as `0x3`, as shown below:

```rust
[package]
name = "Foo"
version = "0.0.0"
[addresses]
C = "0x3"
```

Place the bytecode file and the corresponding  `Move.toml` file in the same directory with the bytecode in a `build` subdirectory. For instance, the layout of the folder `B` (for the package `Bar`) and `C` (for the package `Foo`) would resemble:

```rust
./B
├── Move.toml     
├── build      
 ├ Bar.mv
```

```rust
./C
├── Move.toml   
├── sources 
├── build
  ├── Foo
   ├──bytecode_modules  
     ├ Foo.mv
```

Specify `[dependencies]` in the `Move.toml` of the target (first) package with the location of the dependent (secondary) packages. For instance, assuming all three package directories are at the same level, `Move.toml` of `A` would resemble:

```rust
[package]
name = "A"
version = "0.0.0"
[addresses]
A = "0x2"
[dependencies]
Bar = { local = "../B" }
Foo = { local = "../C" }
```

Note that if both the bytecode and the source code of the same package exist in the search paths, the compiler will complain that the declaration is duplicated. 

Use `aptos move compile` or `aptos move publish` for compiling and publishing the package. See [Use the Aptos CLI](../../cli-tools/aptos-cli-tool/use-aptos-cli.md) for more details.