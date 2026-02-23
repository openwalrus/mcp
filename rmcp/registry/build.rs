use std::{env, fs, path::PathBuf};

fn main() {
    let schema_path = "schemas/server.schema.json";
    println!("cargo:rerun-if-changed={schema_path}");

    let schema_content =
        fs::read_to_string(schema_path).expect("failed to read server.schema.json");
    let schema = serde_json::from_str::<schemars::schema::RootSchema>(&schema_content)
        .expect("failed to parse JSON schema");

    let mut type_space = typify::TypeSpace::default();
    type_space
        .add_root_schema(schema)
        .expect("failed to process schema");

    let tokens = type_space.to_stream();
    let ast = syn::parse2::<syn::File>(tokens).expect("failed to parse generated tokens");
    let content = prettyplease::unparse(&ast);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("server_schema.rs"), content).expect("failed to write generated code");
}
