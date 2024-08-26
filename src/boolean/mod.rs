use argmin::core::ArgminFloat;
use itertools::Itertools;
use nalgebra::{allocator::Allocator, Const, DefaultAllocator, DimName};

use crate::{
    curve::NurbsCurve,
    misc::FloatingPoint,
    prelude::{Contains, CurveIntersectionSolverOptions},
    region::{CompoundCurve, Region},
};

pub enum BooleanOperation {
    Union,
    Intersection,
    Difference,
}

/// A trait for boolean operations.
pub trait Boolean<T> {
    type Output;
    type Option;

    fn union(&self, other: T, option: Self::Option) -> Self::Output;
    fn intersection(&self, other: T, option: Self::Option) -> Self::Output;
    fn difference(&self, other: T, option: Self::Option) -> Self::Output;
    fn boolean(&self, operation: BooleanOperation, other: T, option: Self::Option) -> Self::Output;
}

impl<'a, T: FloatingPoint + ArgminFloat> Boolean<&'a NurbsCurve<T, Const<3>>>
    for NurbsCurve<T, Const<3>>
where
    DefaultAllocator: Allocator<Const<3>>,
{
    type Output = anyhow::Result<Vec<Region<T>>>;
    type Option = Option<CurveIntersectionSolverOptions<T>>;

    fn union(&self, other: &'a NurbsCurve<T, Const<3>>, option: Self::Option) -> Self::Output {
        self.boolean(BooleanOperation::Union, other, option)
    }

    fn intersection(
        &self,
        other: &'a NurbsCurve<T, Const<3>>,
        option: Self::Option,
    ) -> Self::Output {
        self.boolean(BooleanOperation::Intersection, other, option)
    }

    fn difference(&self, other: &'a NurbsCurve<T, Const<3>>, option: Self::Option) -> Self::Output {
        self.boolean(BooleanOperation::Difference, other, option)
    }

    fn boolean(
        &self,
        operation: BooleanOperation,
        other: &'a NurbsCurve<T, Const<3>>,
        option: Self::Option,
    ) -> Self::Output {
        let mut intersections = self.find_intersections(other, option.clone())?;
        if intersections.is_empty() {
            anyhow::bail!("Todo: no intersections case");
        }

        intersections.sort_by(|i0, i1| i0.a().1.partial_cmp(&i1.a().1).unwrap());

        anyhow::ensure!(
            intersections.len() % 2 == 0,
            "Odd number of intersections found"
        );

        let mut regions = vec![];

        let start = self.point_at(self.knots_domain().0);
        let other_contains_self_start = other.contains(&start, option.clone())?;
        let mut curves = [self, other].into_iter().enumerate().cycle();

        match operation {
            BooleanOperation::Union => {
                let cycled = intersections
                    .iter()
                    .cycle()
                    .take(intersections.len() + 1)
                    .collect_vec();
                let windows = cycled.windows(2);

                let mut spans = vec![];

                if !other_contains_self_start {
                    curves.next();
                }

                for it in windows {
                    let (i0, i1) = (&it[0], &it[1]);
                    let c = curves.next();
                    if let Some((idx, c)) = c {
                        let params = match idx % 2 {
                            0 => (i0.a().1, i1.a().1),
                            1 => (i0.b().1, i1.b().1),
                            _ => unreachable!(),
                        };
                        if idx % 2 == 0 {
                            let s = try_trim(c, params)?;
                            spans.extend(s);
                        } else {
                            let s = try_trim(c, params)?;
                            spans.extend(s);
                        }
                    }
                }

                regions.push(Region::new(CompoundCurve::from_iter(spans), vec![]));
            }
            BooleanOperation::Intersection => {
                let cycled = intersections
                    .iter()
                    .cycle()
                    .take(intersections.len() + 1)
                    .collect_vec();
                let windows = cycled.windows(2);

                let mut spans = vec![];

                if other_contains_self_start {
                    curves.next();
                }

                for it in windows {
                    let (i0, i1) = (&it[0], &it[1]);
                    let c = curves.next();
                    if let Some((idx, c)) = c {
                        let params = match idx % 2 {
                            0 => (i0.a().1, i1.a().1),
                            1 => (i0.b().1, i1.b().1),
                            _ => unreachable!(),
                        };
                        if idx % 2 == 0 {
                            let s = try_trim(c, params)?;
                            spans.extend(s);
                        } else {
                            let s = try_trim(c, params)?;
                            spans.extend(s);
                        }
                    }
                }

                regions.push(Region::new(CompoundCurve::from_iter(spans), vec![]));
            }
            BooleanOperation::Difference => {
                let skip_count = if other_contains_self_start { 0 } else { 1 };
                let n = intersections.len();
                let cycled = intersections
                    .into_iter()
                    .cycle()
                    .skip(skip_count)
                    .take(n)
                    .collect_vec();
                let chunks = cycled.chunks(2);
                for it in chunks {
                    let (i0, i1) = (&it[0], &it[1]);
                    let s0 = try_trim(self, (i0.a().1, i1.a().1))?;
                    let s1 = try_trim(other, (i0.b().1, i1.b().1))?;
                    let exterior = [s0, s1].concat();
                    regions.push(Region::new(CompoundCurve::from_iter(exterior), vec![]));
                }
            }
        }

        Ok(regions)
    }
}

fn try_trim<T: FloatingPoint, D: DimName>(
    curve: &NurbsCurve<T, D>,
    parameters: (T, T),
) -> anyhow::Result<Vec<NurbsCurve<T, D>>>
where
    DefaultAllocator: Allocator<D>,
{
    let (min, max) = (
        parameters.0.min(parameters.1),
        parameters.0.max(parameters.1),
    );
    let inside = if parameters.0 < parameters.1 {
        true
    } else {
        false
    };
    let curves = if inside {
        let (_, tail) = curve.try_trim(min)?;
        let (head, _) = tail.try_trim(max)?;
        vec![head]
    } else {
        let (head, tail) = curve.try_trim(min)?;
        let (_, tail2) = tail.try_trim(max)?;
        vec![tail2, head]
    };

    Ok(curves)
}
