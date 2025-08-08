{ pkgs, lib, config, rust-utils }:

let
  inherit (config.project) name;
  
  rustToolchain = pkgs.rust-bin.stable."${config.rust.version}".default.override {
    extensions = config.rust.extensions;
  };

  shellHook = ''
    echo "Rust version: $(rustc --version)"
    echo "Project: ${name} v${config.project.version}"
  '';

in rec {
  shell = pkgs.mkShell {
    buildInputs = with pkgs; [
      rustToolchain
      pkg-config
      clang
    ] ++ lib.optionals stdenv.isDarwin [
      libiconv
    ];

    inherit shellHook;
  };
}