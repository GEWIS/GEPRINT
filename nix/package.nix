{ lib
, stdenv
, rustToolchain
, rustPlatform
, dioxus-cli
, wasm-bindgen-cli
, binaryen
, pkg-config
, openssl
, cacert
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "gewisprint";
  version = "0.1.0";

  src = lib.cleanSource ../.;

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    rustToolchain
    dioxus-cli
    wasm-bindgen-cli
    binaryen
    pkg-config
    rustPlatform.cargoSetupHook
  ];

  buildInputs = [ openssl ];

  # dx shells out to cargo; keep everything offline & deterministic.
  env = {
    OPENSSL_NO_VENDOR = "1";
    SSL_CERT_FILE = "${cacert}/etc/ssl/certs/ca-bundle.crt";
  };

  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR
    # Produce flattened bundle: server binary + public/ web assets.
    dx bundle --release --platform web --out-dir "$PWD/dist"
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/share/gewisprint
    # dx flattens the server executable and the public/ dir into --out-dir.
    cp dist/server $out/bin/gewisprint
    cp -r dist/public $out/share/gewisprint/public
    runHook postInstall
  '';

  meta = {
    description = "Dioxus fullstack CUPS print server";
    mainProgram = "gewisprint";
    platforms = lib.platforms.linux;
  };
})
