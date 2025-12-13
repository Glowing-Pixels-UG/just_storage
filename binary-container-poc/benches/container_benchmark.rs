// SPDX-License-Identifier: MIT
//! Benchmark comparing Binary Document Container (BDC) vs ZIP-based document bundler

use binary_container_poc::{ContainerReader, ContainerWriter};
use criterion::{criterion_group, criterion_main, Criterion};
use document_bundler::{BundleBuilder, BundleReader, BundleWriter};
use std::hint::black_box;
use std::io::Write;
use tempfile::NamedTempFile;

fn create_test_data() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    // Create realistic test data using document bundler types
    let doc = document_bundler::DocumentInfo::new(
        "benchmark_document.pdf".to_string(),
        "benchmark-scanner".to_string(),
        "performance-test".to_string(),
    );

    let metadata = document_bundler::BundleMetadata::new(doc);
    let metadata_json = serde_json::to_vec(&metadata).unwrap();

    // 1MB PDF-like data
    let asset = vec![0xFF; 1024 * 1024];

    // 100KB text data
    let text = vec![b'A'; 100 * 1024];

    // 512KB embeddings data (float32 * 1536 dimensions * 100 vectors)
    let embeddings = vec![0x00; 512 * 1024];

    (metadata_json, asset, text, embeddings)
}

fn benchmark_write_bdc(c: &mut Criterion) {
    let (metadata, asset, text, embeddings) = create_test_data();

    c.bench_function("bdc_write", |b| {
        b.iter(|| {
            let mut writer = ContainerWriter::new();
            writer.add_metadata(black_box(metadata.clone())).unwrap();
            writer.add_asset(black_box(asset.clone())).unwrap();
            writer.add_text(black_box(text.clone())).unwrap();
            writer
                .add_embeddings(black_box(embeddings.clone()))
                .unwrap();
            let _result = writer.finalize().unwrap();
        })
    });
}

fn benchmark_write_zip(c: &mut Criterion) {
    let (metadata_json, asset, text, embeddings) = create_test_data();
    let metadata: document_bundler::BundleMetadata =
        serde_json::from_slice(&metadata_json).unwrap();
    let doc = document_bundler::DocumentInfo::new(
        "benchmark_document.pdf".to_string(),
        "benchmark-scanner".to_string(),
        "performance-test".to_string(),
    );
    let metadata = document_bundler::BundleMetadata::new(doc);

    c.bench_function("zip_write", |b| {
        b.iter(|| {
            let bundle = BundleBuilder::new()
                .metadata(black_box(metadata.clone()))
                .asset(document_bundler::BundleFile::with_data(
                    "asset.pdf".into(),
                    black_box(asset.clone()),
                    "application/pdf".to_string(),
                ))
                .text(document_bundler::BundleFile::with_data(
                    "text.txt".into(),
                    black_box(text.clone()),
                    "text/plain".to_string(),
                ))
                .embeddings(document_bundler::BundleFile::with_data(
                    "embeddings.parquet".into(),
                    black_box(embeddings.clone()),
                    "application/x-parquet".to_string(),
                ))
                .build()
                .unwrap();

            let mut temp_file = NamedTempFile::new().unwrap();
            let writer = BundleWriter::new();
            writer.write(&bundle, temp_file.path()).unwrap();
        })
    });
}

fn benchmark_read_bdc(c: &mut Criterion) {
    let (metadata, asset, text, embeddings) = create_test_data();

    // Pre-create BDC container
    let mut writer = ContainerWriter::new();
    writer.add_metadata(metadata).unwrap();
    writer.add_asset(asset).unwrap();
    writer.add_text(text).unwrap();
    writer.add_embeddings(embeddings).unwrap();
    let bdc_data = writer.finalize().unwrap();

    c.bench_function("bdc_read_full", |b| {
        b.iter(|| {
            let reader = ContainerReader::from_slice(black_box(&bdc_data)).unwrap();
            let _metadata = reader.metadata().unwrap();
            let _asset = reader.asset().unwrap();
            let _text = reader.text().unwrap();
            let _embeddings = reader.embeddings().unwrap();
        })
    });
}

fn benchmark_read_zip(c: &mut Criterion) {
    let (metadata_json, asset, text, embeddings) = create_test_data();
    let metadata: document_bundler::BundleMetadata =
        serde_json::from_slice(&metadata_json).unwrap();

    // Pre-create ZIP bundle
    let bundle = BundleBuilder::new()
        .metadata(metadata)
        .asset(document_bundler::BundleFile::with_data(
            "asset.pdf".into(),
            asset,
            "application/pdf".to_string(),
        ))
        .text(document_bundler::BundleFile::with_data(
            "text.txt".into(),
            text,
            "text/plain".to_string(),
        ))
        .embeddings(document_bundler::BundleFile::with_data(
            "embeddings.parquet".into(),
            embeddings,
            "application/x-parquet".to_string(),
        ))
        .build()
        .unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let writer = BundleWriter::new();
    writer.write(&bundle, temp_file.path()).unwrap();

    c.bench_function("zip_read_full", |b| {
        b.iter(|| {
            let reader = BundleReader::new();
            let extracted = reader.read(black_box(temp_file.path())).unwrap();
            let _metadata = extracted.metadata;
            let _asset = extracted.asset;
            let _text = extracted.text;
            let _embeddings = extracted.embeddings;
        })
    });
}

fn benchmark_read_metadata_only_bdc(c: &mut Criterion) {
    let (metadata, asset, text, embeddings) = create_test_data();

    let mut writer = ContainerWriter::new();
    writer.add_metadata(metadata).unwrap();
    writer.add_asset(asset).unwrap();
    writer.add_text(text).unwrap();
    writer.add_embeddings(embeddings).unwrap();
    let bdc_data = writer.finalize().unwrap();

    c.bench_function("bdc_read_metadata_only", |b| {
        b.iter(|| {
            let reader = ContainerReader::from_slice(black_box(&bdc_data)).unwrap();
            let _metadata = reader.metadata().unwrap();
        })
    });
}

fn benchmark_read_metadata_only_zip(c: &mut Criterion) {
    let (metadata_json, asset, text, embeddings) = create_test_data();
    let metadata: document_bundler::BundleMetadata =
        serde_json::from_slice(&metadata_json).unwrap();

    let bundle = BundleBuilder::new()
        .metadata(metadata)
        .asset(document_bundler::BundleFile::with_data(
            "asset.pdf".into(),
            asset,
            "application/pdf".to_string(),
        ))
        .text(document_bundler::BundleFile::with_data(
            "text.txt".into(),
            text,
            "text/plain".to_string(),
        ))
        .embeddings(document_bundler::BundleFile::with_data(
            "embeddings.parquet".into(),
            embeddings,
            "application/x-parquet".to_string(),
        ))
        .build()
        .unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let writer = BundleWriter::new();
    writer.write(&bundle, temp_file.path()).unwrap();

    c.bench_function("zip_read_metadata_only", |b| {
        b.iter(|| {
            let reader = BundleReader::new();
            let _metadata = reader
                .read_metadata_only(black_box(temp_file.path()))
                .unwrap();
        })
    });
}

fn benchmark_file_sizes(c: &mut Criterion) {
    let (metadata, asset, text, embeddings) = create_test_data();

    // BDC size
    let mut writer = ContainerWriter::new();
    writer.add_metadata(metadata.clone()).unwrap();
    writer.add_asset(asset.clone()).unwrap();
    writer.add_text(text.clone()).unwrap();
    writer.add_embeddings(embeddings.clone()).unwrap();
    let bdc_data = writer.finalize().unwrap();

    // ZIP size
    let doc = document_bundler::DocumentInfo::new(
        "benchmark_document.pdf".to_string(),
        "benchmark-scanner".to_string(),
        "performance-test".to_string(),
    );
    let zip_metadata = document_bundler::BundleMetadata::new(doc);
    let bundle = BundleBuilder::new()
        .metadata(zip_metadata)
        .asset(document_bundler::BundleFile::with_data(
            "asset.pdf".into(),
            asset,
            "application/pdf".to_string(),
        ))
        .text(document_bundler::BundleFile::with_data(
            "text.txt".into(),
            text,
            "text/plain".to_string(),
        ))
        .embeddings(document_bundler::BundleFile::with_data(
            "embeddings.parquet".into(),
            embeddings,
            "application/x-parquet".to_string(),
        ))
        .build()
        .unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let writer = BundleWriter::new();
    writer.write(&bundle, temp_file.path()).unwrap();
    let zip_size = std::fs::metadata(temp_file.path()).unwrap().len();

    c.bench_function("file_sizes", |b| {
        b.iter(|| {
            black_box(bdc_data.len());
            black_box(zip_size);
        })
    });

    println!("BDC size: {} bytes", bdc_data.len());
    println!("ZIP size: {} bytes", zip_size);
    println!(
        "BDC overhead: {:.2}%",
        (bdc_data.len() as f64 / (bdc_data.len() + 1024 * 1024 + 100 * 1024 + 512 * 1024) as f64)
            * 100.0
    );
    println!(
        "ZIP overhead: {:.2}%",
        (zip_size as f64 / (zip_size + 1024 * 1024 + 100 * 1024 + 512 * 1024) as f64) * 100.0
    );
}

criterion_group!(
    benches,
    benchmark_write_bdc,
    benchmark_write_zip,
    benchmark_read_bdc,
    benchmark_read_zip,
    benchmark_read_metadata_only_bdc,
    benchmark_read_metadata_only_zip,
    benchmark_file_sizes
);
criterion_main!(benches);
