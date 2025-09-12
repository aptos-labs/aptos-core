{ pkgs ? import <nixpkgs> { } }:

{
  aptos-core = pkgs.callPackage ./pkgs/aptos-node.nix { };
  aptos-core-docker = pkgs.callPackage ./pkgs/aptos-core-docker.nix {
    aptos-core = pkgs.callPackage ./pkgs/aptos-node.nix { };
  };
}