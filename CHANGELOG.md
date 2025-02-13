# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to Rust's notion of
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `pasta_curves::arithmetic::SqrtRatio` trait, extending `ff::PrimeField` with
  square roots of ratios. This trait is likely to be moved into the `ff` crate
  in a future release (once we're satisfied with it).

### Removed
- `pasta_curves::arithmetic`:
  - `Field` re-export (`pasta_curves::group::ff::Field` is equivalent).
  - `FieldExt::ROOT_OF_UNITY` (use `ff::PrimeField::root_of_unity` instead).
  - `FieldExt::{T_MINUS1_OVER2, pow_by_t_minus1_over2, get_lower_32, sqrt_alt,`
    `sqrt_ratio}` (moved to `SqrtRatio` trait).
  - `FieldExt::{RESCUE_ALPHA, RESCUE_INVALPHA}`
  - `FieldExt::from_u64` (use `From<u64> for ff::PrimeField` instead).
  - `FieldExt::{from_bytes, read, to_bytes, write}`
    (use `ff::PrimeField::{from_repr, to_repr}` instead).

## [0.2.1] - 2021-09-17
### Changed
- The crate is now licensed as `MIT OR Apache-2.0`.

## [0.2.0] - 2021-09-02
### Changed
- Migrated to `ff 0.11`, `group 0.11`.

## [0.1.2] - 2021-08-06
### Added
- Implementation of `group::WnafGroup` for Pallas and Vesta, enabling them to be
  used with `group::Wnaf` for targeted performance improvements.

## [0.1.1] - 2021-06-04
### Added
- Implementations of `group::cofactor::{CofactorCurve, CofactorCurveAffine}` for
  Pallas and Vesta, enabling them to be used in cofactor-aware protocols that
  also want to leverage the affine point representation.

## [0.1.0] - 2021-06-01
Initial release!
