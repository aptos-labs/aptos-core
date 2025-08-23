{ pkgs ? import <nixpkgs> { }
, aptos-core
}:

pkgs.dockerTools.buildLayeredImage {
  name = "aptos-node";
  tag = "latest";

  contents = [
    pkgs.caCertificates
    pkgs.tzdata
    aptos-core
  ];

  config = {
    Cmd = [ "/bin/aptos-node" "--help" ];
    Entrypoint = [ "/bin/aptos-node" ];
    ExposedPorts = {
      "8080/tcp" = { };
      "9101/tcp" = { };
      "6180/tcp" = { };
    };
  };
}