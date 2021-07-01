#!/usr/bin/env nix-shell
with import <nixpkgs> { overlays = [ ]; };

let
  my-packages = [
    cargo
    rustc
    pkg-config
    openssl
    openssl.dev
    pkg-config
  ];
in
mkShell {
  RUSTFLAGS = "-C target-cpu=native";
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  buildInputs = [
    my-packages
  ];
}
