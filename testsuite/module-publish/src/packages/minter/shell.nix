with import <nixpkgs> { };

pkgs.mkShell {
  buildInputs = [
    jq
    nodePackages.nodemon
    nodejs_18
    (callPackage ../aptos.nix { })
  ];

  shellHook = ''
    alias gen="aptos init"

    test() {
      nodemon \
        --ignore build/* \
        --ext move \
        --exec "aptos move test --dev --skip-fetch-latest-git-deps;"
    }

    pub() {
      local minter=0x$(aptos config show-profiles | jq -r '.Result.default.account')
      aptos move publish \
        --named-addresses minter=$minter \
        --skip-fetch-latest-git-deps
    }
  '';
}
