//! Benchmarks de operações do ssh-cli.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ssh_cli::mascaramento::mascarar;
use ssh_cli::paths::{normalizar_nfc, validar_e_normalizar, validar_nome};

fn bench_mascaramento(c: &mut Criterion) {
    c.bench_function("mascarar_short", |b| {
        b.iter(|| mascarar(black_box("curto")))
    });
    c.bench_function("mascarar_long", |b| {
        b.iter(|| mascarar(black_box("senha-secreta-muito-longa-aqui-123456")))
    });
    c.bench_function("mascarar_unicode", |b| {
        b.iter(|| mascarar(black_box("ação você está configuração Itaú")))
    });
}

fn bench_paths(c: &mut Criterion) {
    c.bench_function("validar_nome", |b| {
        b.iter(|| validar_nome(black_box("meu-servidor-producao")))
    });
    c.bench_function("normalizar_nfc_nfd", |b| {
        b.iter(|| normalizar_nfc(black_box("cafe\u{0301}")))
    });
    c.bench_function("normalizar_nfc_noop", |b| {
        b.iter(|| normalizar_nfc(black_box("servidor")))
    });
    c.bench_function("validar_e_normalizar", |b| {
        b.iter(|| validar_e_normalizar(black_box("meu-servidor")))
    });
}

criterion_group!(benches, bench_mascaramento, bench_paths);
criterion_main!(benches);
