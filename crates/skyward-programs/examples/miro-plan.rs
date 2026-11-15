use skyward_programs::{phone::{Phone, Partition, PARTITIONS}, secure};
use std::{collections::HashMap, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let base=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/described/miro");
    let parts=PARTITIONS.iter().map(|name|Partition{
        name:name.to_string(),at:base.join(name),mounted:match *name{"vendor"|"odm"=>format!("/{name}"),_=>format!("/mnt/{name}")}.into()
    }).collect();
    let props=HashMap::from([
        ("ro.boot.hardware".into(),"qcom".into()),
        ("ro.boot.product.hardware.sku".into(),"miro".into()),
        ("ro.boot.product.vendor.sku".into(),"sun".into()),
        ("ro.board.platform".into(),"sun".into()),
    ]);
    let phone=Phone::read(parts,props,None)?;
    for step in secure::steps(&phone)? { println!("{step:#?}"); }
    Ok(())
}
