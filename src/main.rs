use async_recursion::async_recursion;
use clap::Parser;
use excel::*;
use serde_json::Value;
use std::{sync::Mutex, vec};
extern crate simple_excel_writer as excel;

#[tokio::main]
async fn main() {
    let args = ArgOption::parse();
    // let data_out = true;
    // let data_in = false;
    *TOP_N.lock().unwrap() = args.top_n as usize;
    let mut token = String::from("f715c0e9-ea32-4f7c-9602-2aa8c02cfa88");
    if let Some(token_tmp) = args.token {
        token = token_tmp;
    }
    let address_layer = layer_get_address(&args.address, args.height, &token).await;
    match &address_layer {
        AddressLayer::Node(_, vec) => {
            if vec.len() == 0 {
                println!("No data obtained!!!");
                return;
            }
        }
    }
    write_xlsx(address_layer, &args.address, args.height).await;
}

async fn write_xlsx(address_layer: AddressLayer, address: &String, height: i32) {
    let mut wb = Workbook::create(format!("{}.xlsx", address).as_str());
    let mut sheet = wb.create_sheet("Sheet1");
    for _i in 0..height + 1 {
        sheet.add_column(Column { width: 50.0 });
    }
    wb.write_sheet(&mut sheet, |sheet_writer| {
        let sw: &mut SheetWriter<'_, '_> = sheet_writer;
        match address_layer {
            AddressLayer::Node(addr, vec) => {
                sw_write(addr, vec, sw, 1);
            }
        }
        Ok(())
    })
    .expect("write excel error!");
    wb.close().expect("close excel error!");
    println!("Success!!!");
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ArgOption {
    /// 地址
    #[arg(short = 'A', long)]
    address: String,
    /// 层级
    #[arg(short = 'H', long, default_value_t = 3)]
    height: i32,
    /// 指定每条地址返回topN
    #[arg(short = 'T', long, default_value_t = 5)]
    top_n: i32,
    /// 修改token
    #[arg(long)]
    token: Option<String>,

}

fn sw_write(
    address: String,
    address_list: Box<Vec<AddressLayer>>,
    sw: &mut SheetWriter<'_, '_>,
    layer: i32,
) {
    for (idx, address_layer) in address_list.into_iter().enumerate() {
        match address_layer {
            AddressLayer::Node(children_address, children_address_list) => {
                if idx == 0 {
                    let mut row_1 = row![];
                    row_1.add_empty_cells((layer - 1) as usize);
                    row_1.add_cell(address.clone());
                    row_1.add_cell(children_address);
                    sw.append_row(row_1).unwrap();
                    continue;
                };
                if children_address_list.len() == 0 {
                    let mut row_1 = row![];
                    row_1.add_empty_cells(layer as usize);
                    row_1.add_cell(children_address);
                    sw.append_row(row_1).unwrap();
                    continue;
                }
                sw_write(children_address, children_address_list, sw, layer + 1);
            }
        }
    }
}

#[derive(Debug)]
enum AddressLayer {
    Node(String, Box<Vec<AddressLayer>>),
}

#[async_recursion]
async fn layer_get_address(address: &String, layer: i32, token: &String) -> AddressLayer {
    let address_vec = get_address(address, token).await;
    let mut vec = Vec::new();
    for add in address_vec {
        if layer == 1 {
            vec.push(AddressLayer::Node(add, Box::default()));
        } else {
            vec.push(layer_get_address(&add, layer - 1, token).await);
        }
    }
    AddressLayer::Node(address.to_string(), Box::new(vec))
}

async fn get_address(address: &String, token: &String) -> Vec<String> {
    let url = format!(
        "{}?address={}&direction={}&sortBy={}",
        URL, address, DIRECTION, SORT_BY
    );
    let mut result_vec = vec![];
    match HTTPS_CLIENT
        .get(url)
        .header("TRON-PRO-API-KEY", token)
        .send()
        .await
    {
        Ok(data) => {
            if data.status() != reqwest::StatusCode::OK {
                println!("Api not available");
                return vec![];
            }
            let data_str = match data.text().await {
                Ok(data) => data,
                Err(_) => return vec![],
            };
            let data_value: Value = match serde_json::from_str(&data_str) {
                Ok(data) => data,
                Err(_) => return vec![],
            };
            match data_value["code"].as_i64() {
                Some(code) => {
                    if code == 1 {
                        println!("Address is invalid");
                        return vec![];
                    }
                }
                None => {
                    println!("Address is invalid");
                    return vec![];
                }
            }
            let transfer_out: &Vec<Value> = match data_value["transfer_out"]["data"].as_array() {
                Some(data) => data,
                None => return vec![],
            };
            for transfer_out_data in transfer_out {
                if result_vec.len() < *TOP_N.lock().unwrap() {
                    let address = transfer_out_data["address"].as_str().unwrap().to_string();
                    result_vec.push(address);
                }
            }
        }
        Err(_) => {}
    }
    return result_vec;
}

lazy_static::lazy_static! {
    static ref HTTPS_CLIENT: reqwest::Client = reqwest::Client::builder()
        .tls_built_in_root_certs(true)
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .unwrap();
    static ref TOP_N: Mutex<usize> = Mutex::new(0);
}

// const ADDRESS: &str = "TWkjbLiCgVmj8RZjdPK5bVS3xmhvzS9opD";
const DIRECTION: i32 = 2; //0全部，1 转入，2转出
const URL: &str = "https://apilist.tronscanapi.com/api/deep/account/transferAmount";
const SORT_BY: &str = "amountIn";
