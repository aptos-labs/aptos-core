{ pkgs ? import <nixpkgs> { } }:

{
  aptos-core = pkgs.callPackage ./pkgs/aptos-core.nix { };
  aptos-core-docker = pkgs.callPackage ./pkgs/aptos-core-docker.nix {
    aptos-core = pkgs.callPackage ./pkgs/aptos-core.nix { };
  };
}