{
  description = "Ainux Industrial Kernel Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        
        # Cross-compilation toolchain for x86_64-unknown-none
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          targets = [ "x86_64-unknown-none" ];
          extensions = [ "rust-src" "llvm-tools-preview" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            qemu
            ovmf
            xorriso
            grub2
            mtools
            gdb
            nasm
          ];

          shellHook = ''
            export KERNEL_NAME="Ainux"
            export PS1="\[\033[1;36m\](nix-ainux) \[\033[0m\]\w \$ "
            echo "╔══════════════════════════════════════════════════╗"
            echo "║       AINUX NIX INDUSTRIAL ENVIRONMENT       ║"
            echo "╚══════════════════════════════════════════════════╝"
          '';
        };
      }
    );
}
