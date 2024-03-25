use const_format::concatcp;
use futures::{stream, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub result: Vec<RecordDetail>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordDetail {
    pub allow_d3: String,
    pub allow_s1: String,
    pub allow_s2: String,
    pub allow_sma: String,
    pub check_certificate: String,
    pub highest_age_d3: Option<String>,
    pub highest_age_s1: Option<String>,
    pub highest_age_s2: Option<String>,
    pub highest_age_sma: Option<String>,
    pub jenjang: Value,
    pub jurusan_filter_type: String,
    pub jurusan_name: Value,
    pub lowest_ipk_d3: Option<String>,
    pub lowest_ipk_s1: Option<String>,
    pub lowest_ipk_s2: Option<String>,
    pub lowest_ipk_sma: Option<String>,
    pub major_group_non_sma: String,
    pub major_group_sma: String,
    pub major_non_sma_custom: String,
    pub major_sma_custom: String,
    pub stream_name: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub total_job_available: String,
    pub vacancy_base_url: String,
    pub vacancy_id: String,
    pub vacancy_name: String,
    pub vacancy_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detail {
    pub allow_d3: String,
    pub allow_s1: String,
    pub allow_s2: String,
    pub allow_sma: String,
    pub applied: Value,
    pub check_certificate: String,
    pub employee_type_vac: String,
    pub highest_age_d3: Value,
    pub highest_age_s1: String,
    pub highest_age_s2: Value,
    pub highest_age_sma: Value,
    pub jenis_kelamin: String,
    pub job_function: String,
    pub kota_penempatan: Value,
    pub list_certificate: Value,
    pub list_province_text: Value,
    pub logo: String,
    pub lowest_ipk_d3: Value,
    pub lowest_ipk_s1: String,
    pub lowest_ipk_s2: Value,
    pub lowest_ipk_sma: Value,
    pub major_education_filter_type: String,
    pub major_group_non_sma: String,
    pub major_group_sma: String,
    pub no_ig: String,
    pub quota: String,
    pub tenant_name: String,
    pub vacancy_description: String,
    pub vacancy_id: String,
    pub vacancy_name: String,
    pub vacancy_requirements: String,
    pub vacancy_type: String,
}

const URL_BASE: &'static str = "https://rekrutmenbersama2024.fhcibumn.id";
const URL_JOB: &'static str = concatcp!(URL_BASE, "/job");
const URL_LOAD_RECORD: &'static str = concatcp!(URL_BASE, "/job/loadRecord");
const URL_GET_DETAIL: &'static str = concatcp!(URL_BASE, "/job/get_detail_vac");

const WORKER: usize = 16;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/64.0.3282.140 Safari/537.36".parse().unwrap());

    let client = Client::builder()
        .cookie_store(true)
        .default_headers(headers)
        .build()?;

    let resp = client.get(URL_JOB).send().await?;
    let document = Html::parse_document(&resp.text().await?);
    let selector = Selector::parse(r#"input[name="csrf_fhci"]"#).unwrap();
    let csrf_token = document
        .select(&selector)
        .next()
        .ok_or("CSRF token not found")?
        .value()
        .attr("value")
        .ok_or("Value for CSRF token not found")?;

    let resp = client
        .post(URL_LOAD_RECORD)
        .form(&[("csrf_fhci", csrf_token), ("company", "all")])
        .send()
        .await?;

    let jobs = resp.json::<Job>().await?;
    println!("jumlah job {}", jobs.data.result.len());

    serde_json::to_writer_pretty(std::fs::File::create("jobs.json")?, &jobs)?;
    csv::Writer::from_path("jobs.csv")?.serialize(&jobs.data.result)?;

    let detail = stream::iter(jobs.data.result.into_iter().enumerate())
        .map(|(i, job)| {
            println!("fetching job number {} with id {} named {}", i, job.vacancy_id, job.vacancy_name);
            let client = client.clone();
            async move {
                let resp = client
                    .post(URL_GET_DETAIL)
                    .form(&[("csrf_fhci", csrf_token), ("id", job.vacancy_id.as_str())])
                    .send()
                    .await?;

                let detail = resp.json::<Detail>().await?;
                Ok::<_, Box<dyn Error>>(detail)
            }
        })
        .buffer_unordered(WORKER)
        .filter_map(|x| async { x.ok() })
        .collect::<Vec<_>>()
        .await;

    println!("jumlah detail {}", detail.len());

    serde_json::to_writer_pretty(std::fs::File::create("detail.json")?, &detail)?;
    csv::Writer::from_path("detail.csv")?.serialize(&detail)?;

    Ok(())
}
