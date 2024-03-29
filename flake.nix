{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-23.11";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs =
    { self
    , flake-utils
    , crane
    , nixpkgs
    , rust-overlay
    }: flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;

          overlays = [
            (import rust-overlay)
          ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        assetsFilter = path: _type: (builtins.match "^.*/assets/yells_at\.png$" path) != null;
        filter = path: type: (craneLib.filterCargoSources path type) || (assetsFilter path type);

        src = pkgs.lib.cleanSourceWith {
          src = craneLib.path ./.;
          inherit filter;
        };

        systemBuildInputs =
          if pkgs.stdenv.isDarwin
          then
            [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ]
          else
            [ ];
      in rec
      {
        packages.default = craneLib.buildPackage {
          inherit src;
          strictDeps = true;
          doCheck = false; # No tests yet
          nativeBuildInputs = systemBuildInputs;

          RUST_BACKTRACE = 1;
        };

        packages.procfile = pkgs.writeTextFile {
          name = "Procfile";
          text = ''
            web: /bin/yells-at
            nginx: /bin/nginx -c ${./config/nginx.conf}
          '';
        };

        packages.container = pkgs.dockerTools.streamLayeredImage {
          name = "yells-at";
          tag = "latest";

          contents = with pkgs; [ 
            cacert 
            hivemind
            nginx
            packages.default
            fakeNss
          ];

          config.Cmd = [
            "hivemind"
            "${packages.procfile}"
          ];

          extraCommands = ''
            mkdir -p tmp/nginx_cache
            mkdir -p var/log/nginx
          '';
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ rustToolchain ] ++ systemBuildInputs;
          buildInputs = with pkgs; [ hivemind nginx ];
        };
      }
    );
}
