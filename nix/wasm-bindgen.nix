# Pin wasm-bindgen-cli to the version dioxus 0.7.9 requires (0.2.126).
# nixpkgs currently ships 0.2.121, and dx refuses a mismatched CLI.
final: prev:
let
  src = final.fetchCrate {
    pname = "wasm-bindgen-cli";
    version = "0.2.126";
    hash = "sha256-H6Is3fiZVxZCfOMWK5dWMSrtn50VGv0sfdnsT+cTtyk=";
  };
in
{
  wasm-bindgen-cli = final.buildWasmBindgenCli {
    inherit src;
    cargoDeps = final.rustPlatform.fetchCargoVendor {
      hash = "sha256-VucqkXbCi4qtQzY/HrXiDnbSURsagPsdNVMn1Tw3UiY=";
      name = "wasm-bindgen-cli-0.2.126";
    };
  };
}
