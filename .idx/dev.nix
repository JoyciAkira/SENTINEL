# To learn more about how to use Nix to configure your environment
# see: https://firebase.google.com/docs/studio/customize-workspace
{ pkgs, ... }: {
  # Which nixpkgs channel to use.
  channel = "stable-24.05"; 

  # Use https://search.nixos.org/packages to find packages
  packages = [
    pkgs.rustup
    pkgs.cargo
    pkgs.rustc
    pkgs.gcc
    pkgs.pkg-config
    pkgs.openssl
    pkgs.python3
    pkgs.nodejs_20
  ];

  # Sets environment variables in the workspace
  env = {
    RUST_BACKTRACE = "1";
  };
  idx = {
    extensions = [
      "rust-lang.rust-analyzer"
      "serayuzgur.crates"
      "tamasfe.even-better-toml"
    ];

    # Enable previews
    previews = {
      enable = true;
      previews = {
      };
    };

    # Workspace lifecycle hooks
    workspace = {
      onCreate = {
        rust-setup = "rustup default stable";
      };
      onStart = {
      };
    };
  };
}
