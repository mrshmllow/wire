use lib::Hive;

pub async fn inspect(hive: Hive, _online: bool, json: bool) -> Result<(), anyhow::Error> {
    print!(
        "{}",
        match json {
            true => serde_json::to_string_pretty(&hive)?,
            false => format!("{hive:#?}"),
        }
    );

    Ok(())
}
