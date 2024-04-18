# Curvo

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/mattatz/curvo#license)
[![Crates.io](https://img.shields.io/crates/v/curvo.svg)](https://crates.io/crates/curvo)
[![Docs](https://docs.rs/curvo/badge.svg)](https://docs.rs/curvo/latest/curvo/)
[![CI](https://github.com/bevyengine/curvo/workflows/CI/badge.svg)](https://github.com/mattatz/curvo/actions)

Curvo is a NURBS curve / surface modeling library for Rust.

![Visualization on bevy](https://github.com/mattatz/curvo/assets/1085910/50b44a8c-d8c1-43e0-8db5-d6fff52300e6)
*Visualization on [Bevy](https://bevyengine.org/)*

This library enables not only the creation of NURBS curves from control points, knot vectors, and weights associated with each control point, but also supports generating curves that precisely pass through the given control points and creating periodic curves. Additionally, it allows for the construction of NURBS surfaces through operations such as _extruding_ and _lofting_ based on NURBS curves as inputs.

The modeling operations for NURBS surfaces supported by this library currently include (or are planned to include) the following:

- [x] Extrude
- [x] Loft
- [ ] Sweep
- [ ] Revolve

I also plan to implement features for finding the nearest points on curves and surfaces, as well as dividing them based on arc lengths.

## Usage

```rust
// Create a set of points to interpolate
let points = vec![
    Point3::new(-1.0, -1.0, 0.),
    Point3::new(1.0, -1.0, 0.),
    Point3::new(1.0, 1.0, 0.),
    Point3::new(-1.0, 1.0, 0.),
    Point3::new(-1.0, 2.0, 0.),
    Point3::new(1.0, 2.5, 0.),
];

// Create a NURBS curve that interpolates the given points with degree 3
// You can also specify the precision of the curve by generic type (f32 or f64)
let interpolated = NurbsCurve3D::<f64>::try_interpolate(&points, 3, None, None).unwrap();

// NURBS curve & surface can be transformed by nalgebra's matrix
let rotation = Rotation3::from_axis_angle(&Vector3::z_axis(), FRAC_PI_2);
let translation = Translation3::new(0., 0., 3.);
let transform_matrix = rotation * translation; // nalgebra::Isometry3

// Transform the curve by the given matrix (nalgebra::Isometry3 into nalgebra::Matrix4)
let offsetted = interpolated.transformed(&transform_matrix.into());

// Create a NURBS surface by lofting two NURBS curves
let lofted = NurbsSurface::try_loft(
  &[interpolated, offsetted],
  Some(3), // degree of v direction
).unwrap();

// Tessellate the surface in adaptive manner about curvature for efficient rendering
let option = AdaptiveTessellationOptions {
    norm_tolerance: 1e-4,
    ..Default::default()
};
let tessellation = lofted.tessellate(Some(option));

```

## Dependencies

- [nalgebra](https://crates.io/crates/nalgebra): this library heavily relies on nalgebra, a linear algebra library, to perform its computations.

## References

- [The NURBS Book](https://www.amazon.com/NURBS-Book-Monographs-Visual-Communication/dp/3540615458) by Piegl and Tiller