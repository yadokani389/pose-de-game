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
          inherit (pkgs) lib;
          toolchain = pkgs.rust-bin.stable.latest.default;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
          pose-de-game =
            {
              cudaSupport ? false,
              openvinoSupport ? false,
            }:
            rustPlatform.buildRustPackage {
              pname = "pose-de-game";
              version = "0.1.0";

              src = ./.;

              buildFeatures = lib.optional openvinoSupport "openvino";

              nativeBuildInputs = with pkgs; [
                makeWrapper
                pkg-config
              ];

              buildInputs =
                with pkgs;
                [
                  zstd

                  openssl
                  rustPlatform.bindgenHook
                  (onnxruntime.override { inherit cudaSupport; })
                ]
                ++ lib.optionals stdenv.hostPlatform.isLinux [
                  alsa-lib
                  libxkbcommon
                  udev
                  vulkan-loader
                  wayland
                  xorg.libX11
                  xorg.libXcursor
                  xorg.libXi
                  xorg.libXrandr
                ]
                ++ lib.optional openvinoSupport [
                  openvino
                ];

              OPENVINO_INSTALL_DIR = pkgs.lib.optionalString openvinoSupport "${pkgs.openvino}";

              cargoDeps = rustPlatform.importCargoLock {
                lockFile = ./Cargo.lock;
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

          dev =
            {
              cudaSupport ? false,
              openvinoSupport ? false,
            }:
            pkgs.mkShell {
              inputsFrom = [
                config.pre-commit.devShell
                (pose-de-game { inherit cudaSupport openvinoSupport; })
              ];

              LD_LIBRARY_PATH =
                with pkgs;
                lib.optionalString stdenv.hostPlatform.isLinux (
                  lib.makeLibraryPath [
                    libxkbcommon
                    vulkan-loader
                    udev
                    alsa-lib
                    wayland

                    # for detect
                    openssl
                    (onnxruntime.override { inherit cudaSupport; })
                    stdenv.cc.cc.lib
                  ]
                  + (lib.optionalString openvinoSupport ":${openvino}/runtime/lib/intel64")
                );

              OPENVINO_INSTALL_DIR = pkgs.lib.optionalString openvinoSupport "${pkgs.openvino}";
            };
        in
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
            config.allowUnfree = true;
          };

          packages = {
            default = pose-de-game { };
            cuda = pose-de-game { cudaSupport = true; };
            openvino = pose-de-game { openvinoSupport = true; };
            full = pose-de-game {
              cudaSupport = true;
              openvinoSupport = true;
            };
          };

          devShells = {
            default = dev { };
            cuda = dev { cudaSupport = true; };
            openvino = dev { openvinoSupport = true; };
            full = dev {
              cudaSupport = true;
              openvinoSupport = true;
            };
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

          pre-commit.settings = {
            hooks = {
              ripsecrets.enable = true;
              typos.enable = true;
              treefmt.enable = true;
              clippy = {
                enable = true;
                packageOverrides.cargo = toolchain;
                packageOverrides.clippy = toolchain;
              };
            };
          };
        };
    };

  nixConfig = {
    extra-substituters = [
      "https://yadokani389.cachix.org"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "yadokani389.cachix.org-1:xHw9jijQFNDKlNprHbQpXX6cVOUO4m/n2lBfx6Bq4jg="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };
}
