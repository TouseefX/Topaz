//! Regression tests for the Luau bytecode deserializer.
//!
//! Covers the bugs that produced:
//!   failed to deserialize bytecode: unknown constant tag 11 at offset N
//!
//! Root causes:
//!   1. CLASS_SHAPE payload was parsed with the wrong field layout, so the
//!      stream desynced and a later payload byte was misread as a tag.
//!   2. Version >= 11 feedback-vector trailer was never consumed, so the
//!      next proto started mid-stream.
//!   3. LBC_CONSTANT_INTEGER magnitudes are varint64, not varint32.

use luau_lifter::deserializer::{self, bytecode::Bytecode, constant::Constant};

/// Minimal LEB128 encoder for the test fixtures.
fn write_varint(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

fn write_varint64(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Build a single-function chunk with the given constants and optional
/// feedback slots. Version is set to `version`.
fn build_chunk(
    version: u8,
    constants: &[(u8, Vec<u8>)],
    feedback_slots: &[(u8, u32)],
) -> Vec<u8> {
    let mut out = Vec::new();
    // version + types_version
    out.push(version);
    if version >= 4 {
        out.push(1); // types_version = 1 (no userdata mapping)
    }

    // string table: one string "Hello"
    write_varint(&mut out, 1);
    write_varint(&mut out, 5);
    out.extend_from_slice(b"Hello");

    // no userdata mapping for types_version < 3

    // one function
    write_varint(&mut out, 1);

    // -- proto --
    // maxstack, nparams, nups, is_vararg
    out.extend_from_slice(&[1, 0, 0, 0]);
    if version >= 4 {
        out.push(0); // flags
        write_varint(&mut out, 0); // typesize = 0
    }

    // instructions: single RETURN R0 1  (op=0x16, A=0, B=1, C=0)
    // raw word with encode_key 0: opcode in low byte
    write_varint(&mut out, 1); // codesize
    let insn: u32 = 0x16 | (0 << 8) | (1 << 16) | (0 << 24);
    write_u32(&mut out, insn);

    // constants
    write_varint(&mut out, constants.len() as u32);
    for (tag, payload) in constants {
        out.push(*tag);
        out.extend_from_slice(payload);
    }

    // child protos
    write_varint(&mut out, 0);

    // line_defined, debugname
    write_varint(&mut out, 0);
    write_varint(&mut out, 0);

    // no lineinfo, no debuginfo
    out.push(0);
    out.push(0);

    // feedback vector (version >= 11)
    if version >= 11 {
        write_varint(&mut out, feedback_slots.len() as u32);
        for (kind, pc) in feedback_slots {
            out.push(*kind);
            write_varint(&mut out, *pc);
        }
    }

    // main function index
    write_varint(&mut out, 0);

    out
}

#[test]
fn class_shape_is_skipped_correctly() {
    // CLASS_SHAPE payload:
    //   varint class_name (const idx 0)
    //   varint nprops = 2
    //   varint nmethods = 1
    //   3 × varint member names
    let mut payload = Vec::new();
    write_varint(&mut payload, 0); // class name idx
    write_varint(&mut payload, 2); // nprops
    write_varint(&mut payload, 1); // nmethods
    write_varint(&mut payload, 0);
    write_varint(&mut payload, 0);
    write_varint(&mut payload, 0);

    // Also include a plain NIL after CLASS_SHAPE so a desync would surface
    // as "unknown constant tag".
    let constants = vec![
        (10u8, payload), // CLASS_SHAPE
        (0u8, Vec::new()), // NIL
        (1u8, vec![1u8]), // BOOLEAN true
    ];

    let bytes = build_chunk(10, &constants, &[]);
    let chunk = match deserializer::deserialize(&bytes, 1).expect("deserialize") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert_eq!(chunk.functions.len(), 1);
    let consts = &chunk.functions[0].constants;
    assert_eq!(consts.len(), 3);
    assert!(matches!(consts[0], Constant::Nil)); // class shape -> Nil
    assert!(matches!(consts[1], Constant::Nil));
    assert!(matches!(consts[2], Constant::Boolean(true)));
}

#[test]
fn feedback_vector_v11_is_consumed() {
    // A version-11 chunk with a non-empty feedback vector. If the
    // feedback section is not skipped, the main-function index will be
    // misaligned and either parsing fails or main points off the end.
    let mut int_payload = Vec::new();
    int_payload.push(0); // positive
    write_varint64(&mut int_payload, 42);

    let constants = vec![
        (9u8, int_payload), // INTEGER 42
        (0u8, Vec::new()),  // NIL
    ];
    let feedback = vec![(0u8, 0u32), (0u8, 3u32)];

    let bytes = build_chunk(11, &constants, &feedback);
    let chunk = match deserializer::deserialize(&bytes, 1).expect("deserialize v11") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert_eq!(chunk.main, 0);
    assert_eq!(chunk.functions.len(), 1);
    let consts = &chunk.functions[0].constants;
    assert_eq!(consts.len(), 2);
    assert!(matches!(consts[0], Constant::Integer(42)));
    assert!(matches!(consts[1], Constant::Nil));
}

#[test]
fn large_integer_constant_uses_varint64() {
    // Magnitude that does not fit in 32 bits: 1 << 40
    let magnitude: u64 = 1u64 << 40;
    let mut payload = Vec::new();
    payload.push(0); // positive
    write_varint64(&mut payload, magnitude);

    let constants = vec![(9u8, payload)];
    let bytes = build_chunk(8, &constants, &[]);
    let chunk = match deserializer::deserialize(&bytes, 1).expect("deserialize int64") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert!(matches!(
        chunk.functions[0].constants[0],
        Constant::Integer(v) if v == (1i64 << 40)
    ));
}

#[test]
fn class_shape_plus_feedback_no_tag_11() {
    // Combined repro for the reported "unknown constant tag 11" failure:
    // version 11 + CLASS_SHAPE + feedback slots. The old parser either
    // desynced on CLASS_SHAPE or left the feedback bytes unread so the
    // next read saw tag=11.
    let mut class_payload = Vec::new();
    write_varint(&mut class_payload, 0);
    write_varint(&mut class_payload, 1); // 1 prop
    write_varint(&mut class_payload, 1); // 1 method
    write_varint(&mut class_payload, 0);
    write_varint(&mut class_payload, 0);

    let constants = vec![
        (3u8, {
            let mut p = Vec::new();
            write_varint(&mut p, 1); // string idx 1 ("Hello")
            p
        }),
        (10u8, class_payload),
        (2u8, {
            let mut p = Vec::new();
            p.extend_from_slice(&3.5f64.to_le_bytes());
            p
        }),
    ];
    let feedback = vec![(0u8, 1u32)];

    let bytes = build_chunk(11, &constants, &feedback);
    let result = deserializer::deserialize(&bytes, 1);
    assert!(
        result.is_ok(),
        "v11 + class_shape + feedback should parse, got: {:?}",
        result.err()
    );
    let chunk = match result.unwrap() {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert_eq!(chunk.functions[0].constants.len(), 3);
}

#[test]
fn table_with_constants_is_interleaved() {
    // Wire layout: tag 8, varint(2), then for each entry:
    //   varint(key) + i32(value_index)
    // If the parser wrongly reads all keys first, the i32 bytes (e.g.
    // little-endian 1 = 01 00 00 00) get treated as a later constant tag
    // and we either mis-parse or report an unknown tag.
    let mut payload = Vec::new();
    write_varint(&mut payload, 2); // 2 entries
    write_varint(&mut payload, 0); // key 0
    payload.extend_from_slice(&1i32.to_le_bytes()); // value const idx 1
    write_varint(&mut payload, 1); // key 1
    payload.extend_from_slice(&(-1i32).to_le_bytes()); // value = nil sentinel

    // Prepend a couple of string constants the keys can point at.
    let mut s0 = Vec::new();
    write_varint(&mut s0, 1); // string table idx 1
    let mut s1 = Vec::new();
    write_varint(&mut s1, 1);

    let constants = vec![
        (3u8, s0),
        (3u8, s1),
        (8u8, payload), // TABLE_WITH_CONSTANTS
        (0u8, Vec::new()), // trailing NIL — must still parse if stream is aligned
    ];

    let bytes = build_chunk(7, &constants, &[]);
    let chunk = match deserializer::deserialize(&bytes, 1).expect("table_with_constants") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    let consts = &chunk.functions[0].constants;
    assert_eq!(consts.len(), 4);
    match &consts[2] {
        Constant::TableWithConstants(entries) => {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].key, 0);
            assert_eq!(entries[0].value_index, 1);
            assert_eq!(entries[1].key, 1);
            assert_eq!(entries[1].value_index, -1);
        }
        other => panic!("expected TableWithConstants, got {other:?}"),
    }
    assert!(matches!(consts[3], Constant::Nil));
}

#[test]
fn typeinfo_does_not_reject_large_constant_tables() {
    // Proto with typesize>0 and sizek > 100 must still parse. The old
    // heuristic required sizek <= 100 after the typeinfo skip, which
    // rejected real ModuleScripts and then picked a wrong skip.
    let mut out = Vec::new();
    out.push(6); // version
    out.push(1); // types_version
    write_varint(&mut out, 0); // no strings
    write_varint(&mut out, 1); // 1 function

    // proto header
    out.extend_from_slice(&[2, 0, 0, 0]); // maxstack, nparams, nups, vararg
    out.push(0); // flags
    // typeinfo: size 4, four opaque bytes
    write_varint(&mut out, 4);
    out.extend_from_slice(&[0x0A, 0x00, 0x00, 0x00]);

    // 1 instruction: RETURN
    write_varint(&mut out, 1);
    let insn: u32 = 0x16 | (0 << 8) | (1 << 16);
    out.extend_from_slice(&insn.to_le_bytes());

    // 120 NIL constants (sizek > 100)
    write_varint(&mut out, 120);
    for _ in 0..120 {
        out.push(0);
    }

    write_varint(&mut out, 0); // child protos
    write_varint(&mut out, 0); // line_defined
    write_varint(&mut out, 0); // debugname
    out.push(0); // no lineinfo
    out.push(0); // no debuginfo
    write_varint(&mut out, 0); // main

    let chunk = match deserializer::deserialize(&out, 1).expect("large sizek + typeinfo") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert_eq!(chunk.functions[0].constants.len(), 120);
}

/// Exhaustive constant-tag payload sizes. If any arm under- or over-reads,
/// a trailing known NIL will not land on a tag byte and the test fails.
#[test]
fn every_known_constant_tag_roundtrips_stream_alignment() {
    // Build one constant of every known tag, then a sentinel NIL.
    let mut constants: Vec<(u8, Vec<u8>)> = Vec::new();

    // 0 NIL
    constants.push((0, vec![]));
    // 1 BOOLEAN true
    constants.push((1, vec![1]));
    // 2 NUMBER 1.5
    {
        let mut p = Vec::new();
        p.extend_from_slice(&1.5f64.to_le_bytes());
        constants.push((2, p));
    }
    // 3 STRING idx 1
    {
        let mut p = Vec::new();
        write_varint(&mut p, 1);
        constants.push((3, p));
    }
    // 4 IMPORT
    {
        let mut p = Vec::new();
        p.extend_from_slice(&0u32.to_le_bytes());
        constants.push((4, p));
    }
    // 5 TABLE with 2 keys
    {
        let mut p = Vec::new();
        write_varint(&mut p, 2);
        write_varint(&mut p, 0);
        write_varint(&mut p, 1);
        constants.push((5, p));
    }
    // 6 CLOSURE proto 0
    {
        let mut p = Vec::new();
        write_varint(&mut p, 0);
        constants.push((6, p));
    }
    // 7 VECTOR
    {
        let mut p = Vec::new();
        for f in [1.0f32, 2.0, 3.0, 0.0] {
            p.extend_from_slice(&f.to_le_bytes());
        }
        constants.push((7, p));
    }
    // 8 TABLE_WITH_CONSTANTS interleaved
    {
        let mut p = Vec::new();
        write_varint(&mut p, 2);
        write_varint(&mut p, 0);
        p.extend_from_slice(&1i32.to_le_bytes());
        write_varint(&mut p, 1);
        p.extend_from_slice(&(-1i32).to_le_bytes());
        constants.push((8, p));
    }
    // 9 INTEGER large
    {
        let mut p = Vec::new();
        p.push(0);
        write_varint64(&mut p, 1u64 << 40);
        constants.push((9, p));
    }
    // 10 CLASS_SHAPE
    {
        let mut p = Vec::new();
        write_varint(&mut p, 0); // class name
        write_varint(&mut p, 2); // nprops
        write_varint(&mut p, 1); // nmethods
        write_varint(&mut p, 0);
        write_varint(&mut p, 1);
        write_varint(&mut p, 0);
        constants.push((10, p));
    }
    // sentinel
    constants.push((0, vec![]));

    let bytes = build_chunk(10, &constants, &[]);
    let chunk = match deserializer::deserialize(&bytes, 1).expect("all tags") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    let consts = &chunk.functions[0].constants;
    assert_eq!(consts.len(), constants.len());
    assert!(matches!(consts[0], Constant::Nil));
    assert!(matches!(consts[1], Constant::Boolean(true)));
    assert!(matches!(consts[2], Constant::Number(n) if (n - 1.5).abs() < 1e-9));
    assert!(matches!(consts[3], Constant::String(1)));
    assert!(matches!(consts[4], Constant::Import(0)));
    assert!(matches!(consts[5], Constant::Table(ref k) if k == &[0, 1]));
    assert!(matches!(consts[6], Constant::Closure(0)));
    assert!(matches!(consts[7], Constant::Vector(1.0, 2.0, 3.0, 0.0)));
    match &consts[8] {
        Constant::TableWithConstants(e) => {
            assert_eq!(e.len(), 2);
            assert_eq!(e[0].key, 0);
            assert_eq!(e[0].value_index, 1);
            assert_eq!(e[1].key, 1);
            assert_eq!(e[1].value_index, -1);
        }
        other => panic!("expected TableWithConstants, got {other:?}"),
    }
    assert!(matches!(consts[9], Constant::Integer(v) if v == (1i64 << 40)));
    assert!(matches!(consts[10], Constant::Nil)); // class shape -> Nil
    assert!(matches!(consts[11], Constant::Nil)); // sentinel
}

#[test]
fn multi_proto_with_feedback_stays_aligned() {
    // Two protos, version 11, each with a feedback slot. Main = 1.
    // If feedback is not consumed, proto 1 starts mid-stream.
    let mut out = Vec::new();
    out.push(11);
    out.push(1); // types_version
    write_varint(&mut out, 0); // strings
    write_varint(&mut out, 2); // two functions

    for _ in 0..2 {
        out.extend_from_slice(&[1, 0, 0, 0]); // header
        out.push(0); // flags
        write_varint(&mut out, 0); // typesize
        write_varint(&mut out, 1); // codesize
        let insn: u32 = 0x16 | (1 << 16);
        out.extend_from_slice(&insn.to_le_bytes());
        write_varint(&mut out, 1); // sizek
        out.push(0); // NIL
        write_varint(&mut out, 0); // children
        write_varint(&mut out, 0); // line
        write_varint(&mut out, 0); // name
        out.push(0); // no lineinfo
        out.push(0); // no debuginfo
        write_varint(&mut out, 1); // 1 feedback slot
        out.push(0); // kind
        write_varint(&mut out, 0); // pc
    }
    write_varint(&mut out, 1); // main = 1

    let chunk = match deserializer::deserialize(&out, 1).expect("multi proto v11") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert_eq!(chunk.functions.len(), 2);
    assert_eq!(chunk.main, 1);
    assert_eq!(chunk.functions[0].constants.len(), 1);
    assert_eq!(chunk.functions[1].constants.len(), 1);
}

#[test]
fn version12_inlinable_cost_is_consumed() {
    // Without reading the cost varint, the size-prefix snap still works,
    // but offset accounting for the size check would fail if cost were
    // larger than remaining declared padding. Test explicit consumption.
    let mut body = Vec::new();
    body.extend_from_slice(&[1, 0, 0, 0]); // header
    body.push(1 << 3); // flags = LPF_INLINABLE
    write_varint(&mut body, 0); // typesize
    write_varint(&mut body, 1);
    body.extend_from_slice(&(0x16u32 | (1 << 16)).to_le_bytes());
    write_varint(&mut body, 0); // sizek
    write_varint(&mut body, 0); // children
    write_varint(&mut body, 0);
    write_varint(&mut body, 0);
    body.push(0);
    body.push(0);
    // feedback (v11+)
    write_varint(&mut body, 0);
    // cost
    write_varint64(&mut body, 999);

    let mut out = Vec::new();
    out.push(12);
    out.push(1);
    write_varint(&mut out, 0);
    write_varint(&mut out, 1);
    write_varint(&mut out, body.len() as u32);
    out.extend_from_slice(&body);
    write_varint(&mut out, 0); // main

    let chunk = match deserializer::deserialize(&out, 1).expect("v12 cost") {
        Bytecode::Chunk(c) => c,
        other => panic!("expected Chunk, got {other:?}"),
    };
    assert_eq!(chunk.functions.len(), 1);
}
