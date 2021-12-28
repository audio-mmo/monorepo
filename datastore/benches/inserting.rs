use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

#[derive(serde::Serialize, serde::Deserialize)]
struct TestRow {
    primary_key: i64,
    string_col: String,
    json_col: HashMap<String, String>,
}

fn build_test_rows(row_count: usize) -> Vec<TestRow> {
    let mut ret = vec![];

    for i in 0..row_count {
        let mut jmap = HashMap::new();
        jmap.insert("a".into(), "b".into());
        jmap.insert("iteration".into(), format!("{}", i));

        ret.push(TestRow {
            primary_key: i as i64,
            string_col: format!("string{}", i),
            json_col: jmap,
        });
    }

    ret
}

fn build_descriptor(path: &std::path::Path) -> ammo_datastore::DatabaseDescriptor {
    let mut builder = ammo_datastore::DatabaseDescriptorBuilder::new(path.to_path_buf());
    builder
        .add_schema("s".into(), |sb| {
            sb.add_table("t".into(), |tb| {
                tb.add_integer_column("primary_key".into() , true, false)
                    .unwrap();
                tb.add_string_column("string_col".into(), false, false)
                    .unwrap();
                tb.add_json_column("json_col".into()).unwrap();

                Ok(())
            })
            .unwrap();

            sb.add_sql_migration("m1".into(), r#"
            CREATE table {{t}}(primary_key INTEGER PRIMARY KEY, string_col TEXT NOT NULL, json_col TEXT NOT NULL)
            "#.into())?;
            Ok(())
        })
        .unwrap();
    builder.build().unwrap()
}

pub fn benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("inserting");
    for size in [5, 10, 20, 50, 100, 500, 5000] {
        group.throughput(Throughput::Elements(size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let tdir = tempfile::TempDir::new().expect("Should create");
            let mut db = ammo_datastore::Database::open(build_descriptor(tdir.path())).unwrap();
            let rows = build_test_rows(*size as usize);

            b.iter(move || {
                db.patch_table("s", "t", &rows[..]).unwrap();
                db.truncate_all_tables().unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
