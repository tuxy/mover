{
  description = "Raspberry Pi Pico RP2040 Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        pname = "mover_controller";

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "llvm-tools"
          ];
          targets = [ "thumbv6m-none-eabi" ];
        };
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          name = pname;
          src = ./.;

          nativeBuildInputs = [
            rustToolchain
            pkgs.elf2uf2-rs
            pkgs.flip-link
            pkgs.rustPlatform.cargoSetupHook
          ];

          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            src = ./.;
            name = "${pname}-cargo-deps";
            hash = "sha256-KICHwPqOXm3LYtU3T7nXbi2pPtQ+bWqze12ihpHhDSw=";
          };

          dontConfigure = true;
          dontAddHostSuffix = true;

          buildPhase = ''
            cargo build --release --target thumbv6m-none-eabi
          '';

          installPhase = ''
            mkdir -p $out/bin
            cp target/thumbv6m-none-eabi/release/${pname} $out/bin/${pname}.elf
            elf2uf2-rs $out/bin/${pname}.elf $out/bin/${pname}.uf2
          '';
        };

        devShells.default = pkgs.mkShell {
          name = "pico-rust-dev";

          packages = with pkgs; [
            rustToolchain
            elf2uf2-rs
            flip-link
            probe-rs-tools
            picotool
            cargo-generate
            cmake
            pkg-config
            gcc
            picocom
            minicom
          ];

          shellHook = ''
            echo "cargo build --release  - build for RP2040"
            echo "elf2uf2-rs             - convert ELF → UF2"
            echo "probe-rs run           - flash & debug via SWD"
            echo "picotool info          - inspect connected Pico"
          '';
        };
      }
    );
}
