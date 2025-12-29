{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      nixpkgs,
      flake-parts,
      ...
    }:

    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      imports = with inputs; [
        git-hooks.flakeModule
        treefmt-nix.flakeModule
      ];

      perSystem =
        {
          config,
          pkgs,
          system,
          ...
        }:
        let
          toolchain = pkgs.rust-bin.stable.latest.default;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
          pose-de-game = rustPlatform.buildRustPackage {
            pname = "pose-de-game";
            version = "0.1.0";

            src = ./game;

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
            ];

            buildInputs = with pkgs; [
              zstd
              libglvnd
              alsa-lib
              udev
              vulkan-loader
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
            ];

            cargoDeps = rustPlatform.importCargoLock {
              lockFile = ./game/Cargo.lock;
              outputHashes = {
                "bevy-wasm-tasks-0.16.0" = "sha256-8RBYwPmGiiXVkmIrV/n2UhIDEX8UzAwIUZV+PcSog5c=";
              };
            };

            postFixup =
              with pkgs;
              lib.optionalString stdenv.hostPlatform.isLinux ''
                patchelf $out/bin/pose-de-game \
                  --add-rpath ${
                    lib.makeLibraryPath [
                      libxkbcommon
                      vulkan-loader
                    ]
                  }
              '';

            meta = {
              homepage = "https://github.com/yadokani389/pose-de-game";
              license = with pkgs.lib.licenses; [
                asl20
                mit
              ];
              mainProgram = "pose-de-game";
            };
          };
        in
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          packages.default = pose-de-game;

          devShells.default = pkgs.mkShell {
            inputsFrom = [
              config.pre-commit.devShell
              pose-de-game
            ];

            packages = with pkgs; [
              vulkan-headers
              (pkgs.python313.withPackages (ps: [
                ps.ultralytics
                ps.opencv4
                ps.cbor2
                ps.openvino
                ps.onnx
              ]))
            ];

            LD_LIBRARY_PATH =
              with pkgs;
              lib.makeLibraryPath [
                libxkbcommon
                vulkan-loader
                udev
                alsa-lib
                kdePackages.wayland
                stdenv.cc.cc.lib

                # for detect
                libglvnd
                glib
              ];
          };

          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              rustfmt.enable = true;
              taplo.enable = true;
            };

            settings.formatter = {
              taplo.options = [
                "fmt"
                "-o"
                "reorder_keys=true"
              ];
            };
          };

          pre-commit = {
            check.enable = true;
            settings = {
              hooks = {
                ripsecrets.enable = true;
                typos.enable = true;
                treefmt.enable = true;
                clippy = {
                  enable = true;
                  packageOverrides.cargo = toolchain;
                  packageOverrides.clippy = toolchain;
                  settings.extraArgs = "--manifest-path game/Cargo.toml --all-targets";
                };
              };
            };
          };
        };
    };
}
