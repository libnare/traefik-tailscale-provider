rec {
  project = {
    name = "traefik-tailscale-provider";
    version = "0.1.0";
    description = "A Traefik provider for Tailscale services";
    license = "mit";
  };

  rust = {
    version = "1.89.0";
    date = "2025-08-07";
    extensions = [ "rust-src" "rust-analyzer" ];
  };

  supportedSystems = [
    "x86_64-linux" 
    "aarch64-linux" 
    "x86_64-darwin" 
    "aarch64-darwin" 
  ];
}