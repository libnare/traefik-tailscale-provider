{ pkgs, lib, config, src, crane }:

let
  inherit (config.project) name version;
  
  stableToolchain = pkgs.rust-bin.stable."${config.rust.version}".default;
  
  craneLib = (crane.mkLib pkgs).overrideToolchain (_: stableToolchain);
  
  commonArgs = {
    src = craneLib.cleanCargoSource src;
    strictDeps = true;
    
    nativeBuildInputs = with pkgs; [
      pkg-config
    ] ++ lib.optionals pkgs.stdenv.isDarwin [
      libiconv
    ];
  };

  buildPackage =
    craneLib.buildPackage (commonArgs // {
      pname = name;
      version = config.project.version;
      
      doCheck = true;
      
      meta = with lib; {
        description = config.project.description;
        license = licenses.${config.project.license};
      };
    });

in {
  inherit craneLib buildPackage;
}