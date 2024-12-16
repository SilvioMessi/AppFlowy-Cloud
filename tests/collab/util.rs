use std::time::Duration;

use anyhow::Context;
use collab::core::collab::DataSource;
use collab::core::origin::CollabOrigin;
use collab::entity::EncodedCollab;
use collab::preclude::Collab;
use collab_document::blocks::Block;
use collab_document::document::Document;
use collab_document::document_data::default_document_collab_data;
use collab_entity::CollabType;
use nanoid::nanoid;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use redis::aio::ConnectionManager;
use tokio::time::sleep;

#[allow(dead_code)]
pub fn generate_random_bytes(size: usize) -> Vec<u8> {
  let s: String = thread_rng()
    .sample_iter(&Alphanumeric)
    .take(size)
    .map(char::from)
    .collect();
  s.into_bytes()
}

#[allow(dead_code)]
pub fn generate_random_string(len: usize) -> String {
  let rng = thread_rng();
  rng
    .sample_iter(&Alphanumeric)
    .take(len)
    .map(char::from)
    .collect()
}

pub fn make_big_collab_doc_state(object_id: &str, key: &str, value: String) -> Vec<u8> {
  let mut collab = Collab::new_with_origin(CollabOrigin::Empty, object_id, vec![], false);
  collab.insert(key, value);
  collab
    .encode_collab_v1(|_| Ok::<(), anyhow::Error>(()))
    .unwrap()
    .doc_state
    .to_vec()
}

pub struct TestDocumentEditor {
  pub document: Document,
}

impl TestDocumentEditor {
  pub(crate) fn clear(&mut self) {
    let page_id = self.document.get_page_id().unwrap();
    let blocks = self.document.get_block_children_ids(&page_id);
    for block in blocks {
      self.document.delete_block(&block).unwrap();
    }
  }

  pub(crate) fn insert_paragraphs(&mut self, paragraphs: Vec<String>) {
    let page_id = self.document.get_page_id().unwrap();
    let mut prev_id = "".to_string();
    for paragraph in paragraphs {
      let block_id = nanoid!(6);
      let text_id = nanoid!(6);
      let block = Block {
        id: block_id.clone(),
        ty: "paragraph".to_owned(),
        parent: page_id.clone(),
        children: "".to_string(),
        external_id: Some(text_id.clone()),
        external_type: Some("text".to_owned()),
        data: Default::default(),
      };

      self
        .document
        .insert_block(block, Some(prev_id.clone()))
        .unwrap();
      prev_id.clone_from(&block_id);
      self
        .document
        .apply_text_delta(&text_id, format!(r#"[{{"insert": "{}"}}]"#, paragraph));
    }
  }

  pub fn encode_collab(&self) -> EncodedCollab {
    self.document.encode_collab().unwrap()
  }
}

pub fn empty_document_editor(object_id: &str) -> TestDocumentEditor {
  let doc_state = default_document_collab_data(object_id)
    .unwrap()
    .doc_state
    .to_vec();
  let collab = Collab::new_with_source(
    CollabOrigin::Empty,
    object_id,
    DataSource::DocStateV1(doc_state),
    vec![],
    false,
  )
  .unwrap();
  TestDocumentEditor {
    document: Document::open(collab).unwrap(),
  }
}

pub fn alex_software_engineer_story() -> Vec<&'static str> {
  vec![
    "Alex is a software programmer who spends his days coding and solving problems.",
    "Outside of work, he enjoys staying active with sports like tennis, basketball, cycling, badminton, and snowboarding.",
    "He learned tennis while living in Singapore and now enjoys playing with his friends on weekends.",
    "Alex had an unforgettable experience trying two diamond slopes in Lake Tahoe, which challenged his snowboarding skills.",
    "He brought his bike with him when he moved to Singapore, excited to explore the city on two wheels.",
    "Although he hasn’t ridden his bike in Singapore yet, Alex looks forward to cycling through the city’s famous parks and scenic routes.",
    "Alex enjoys the thrill of playing different sports, finding balance between his work and physical activities.",
    "From the adrenaline of snowboarding to the strategic moves on the tennis court, each sport gives him a sense of freedom and excitement.",
  ]
}

pub fn alex_banker_story() -> Vec<&'static str> {
  vec![
    "Alex is a banker who spends most of their time working with numbers.",
    "They don't enjoy sports or physical activities, preferring to relax.",
    "Instead of exercise, Alex finds joy in eating delicious food and",
    "exploring new restaurants. They love trying different cuisines and",
    "discovering new dishes that excite their taste buds.",
    "For Alex, food is a form of relaxation and self-care, a break from",
    "the hectic world of banking. A perfect meal brings them comfort.",
    "Whether dining out or cooking at home, Alex enjoys savoring flavors",
    "and experiencing the joy of good food. It’s their favorite way to unwind.",
    "Though they don’t share the same passion for sports, Alex finds",
    "happiness in the culinary world, where each new dish is an adventure.",
  ]
}

#[allow(dead_code)]
pub fn empty_collab_doc_state(object_id: &str, collab_type: CollabType) -> Vec<u8> {
  match collab_type {
    CollabType::Document => default_document_collab_data(object_id)
      .unwrap()
      .doc_state
      .to_vec(),
    CollabType::Unknown => {
      let collab = Collab::new_with_origin(CollabOrigin::Empty, object_id, vec![], false);
      collab
        .encode_collab_v1(|_| Ok::<(), anyhow::Error>(()))
        .unwrap()
        .doc_state
        .to_vec()
    },
    _ => panic!("collab type not supported"),
  }
}

pub fn test_encode_collab_v1(object_id: &str, key: &str, value: &str) -> EncodedCollab {
  let mut collab = Collab::new_with_origin(CollabOrigin::Empty, object_id, vec![], false);
  collab.insert(key, value);
  collab
    .encode_collab_v1(|_| Ok::<(), anyhow::Error>(()))
    .unwrap()
}

pub async fn redis_client() -> redis::Client {
  let redis_uri = "redis://localhost:6379";
  redis::Client::open(redis_uri)
    .context("failed to connect to redis")
    .unwrap()
}

pub async fn redis_connection_manager() -> ConnectionManager {
  let mut attempt = 0;
  let max_attempts = 5;
  let mut wait_time = 500;
  loop {
    match redis_client().await.get_connection_manager().await {
      Ok(manager) => return manager,
      Err(err) => {
        if attempt >= max_attempts {
          panic!("{:?}", err); // Exceeded maximum attempts, return the last error.
        }
        sleep(Duration::from_millis(wait_time)).await;
        wait_time *= 2;
        attempt += 1;
      },
    }
  }
}

use std::io::{BufReader, Read};

use collab::preclude::MapExt;
use flate2::bufread::GzDecoder;
use serde::Deserialize;
use yrs::{GetString, Text, TextRef};

use client_api_test::CollabRef;

/// (position, delete length, insert content).
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct TestPatch(pub usize, pub usize, pub String);

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct TestTxn {
  // time: String, // ISO String. Unused.
  pub patches: Vec<TestPatch>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct TestScenario {
  #[serde(default)]
  pub using_byte_positions: bool,

  #[serde(rename = "startContent")]
  pub start_content: String,
  #[serde(rename = "endContent")]
  pub end_content: String,

  pub txns: Vec<TestTxn>,
}

impl TestScenario {
  /// Load the testing data at the specified file. If the filename ends in .gz, it will be
  /// transparently uncompressed.
  ///
  /// This method panics if the file does not exist, or is corrupt. It'd be better to have a try_
  /// variant of this method, but given this is mostly for benchmarking and testing, I haven't felt
  /// the need to write that code.
  pub fn open(fpath: &str) -> TestScenario {
    // let start = SystemTime::now();
    // let mut file = File::open("benchmark_data/automerge-paper.json.gz").unwrap();
    let file = std::fs::File::open(fpath).unwrap();

    let mut reader = BufReader::new(file);
    // We could pass the GzDecoder straight to serde, but it makes it way slower to parse for
    // some reason.
    let mut raw_json = vec![];

    if fpath.ends_with(".gz") {
      let mut reader = GzDecoder::new(reader);
      reader.read_to_end(&mut raw_json).unwrap();
    } else {
      reader.read_to_end(&mut raw_json).unwrap();
    }

    let data: TestScenario = serde_json::from_reader(raw_json.as_slice()).unwrap();
    data
  }

  pub async fn execute(&self, collab: CollabRef, step_count: usize) -> String {
    let mut i = 0;
    for t in self.txns.iter().take(step_count) {
      i += 1;
      if i % 10_000 == 0 {
        tracing::trace!("Executed {}/{} steps", i, step_count);
      }
      let mut lock = collab.write().await;
      let collab = lock.borrow_mut();
      let mut txn = collab.context.transact_mut();
      let txt = collab.data.get_or_init_text(&mut txn, "text-id");
      for patch in t.patches.iter() {
        let at = patch.0;
        let delete = patch.1;
        let content = patch.2.as_str();

        if delete != 0 {
          txt.remove_range(&mut txn, at as u32, delete as u32);
        }
        if !content.is_empty() {
          txt.insert(&mut txn, at as u32, content);
        }
      }
    }

    // validate after applying all patches
    let lock = collab.read().await;
    let collab = lock.borrow();
    let txn = collab.context.transact();
    let txt: TextRef = collab.data.get_with_txn(&txn, "text-id").unwrap();
    txt.get_string(&txn)
  }
}
