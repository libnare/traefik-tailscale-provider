{
  description = "Traefik Tailscale Provider";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay }:
    let
      config = import ./nix/config.nix;
      
      forAllSystems = nixpkgs.lib.genAttrs config.supportedSystems;
      
      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        }
      );
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
          inherit (pkgs) lib;
          
          rust-utils = import ./nix/rust.nix { inherit pkgs lib config crane; src = self; };
          
        in
        rec {
          ${config.project.name} = rust-utils.buildPackage;
          default = rust-utils.buildPackage;
        }
      );

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
          lib = pkgs.lib;
          rust-utils = import ./nix/rust.nix { inherit pkgs lib config crane; src = self; };
          
        in {
          default = (import ./nix/dev-shell.nix { 
            inherit pkgs lib config rust-utils; 
          }).shell;
        }
      );

      checks = forAllSystems (system: {
        build = self.packages.${system}.default;
        
        format-check = 
          let pkgs = nixpkgsFor.${system};
          in pkgs.runCommand "format-check" {
            buildInputs = [ pkgs.rustfmt ];
          } ''
            cd ${self}
            if [ -f rustfmt.toml ] || [ -f .rustfmt.toml ]; then
              find . -name "*.rs" -exec rustfmt --check {} \;
            fi
            touch $out
          '';
      });
    };
}