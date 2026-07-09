fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    print!("{}", sql_lens_api::openapi_yaml()?);
    Ok(())
}
