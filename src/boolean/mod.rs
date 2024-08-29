use std::{cell::RefCell, rc::Rc};

use argmin::core::ArgminFloat;
use degeneracies::find_intersections_without_degeneracies;
use itertools::Itertools;
use nalgebra::{allocator::Allocator, Const, DefaultAllocator, DimName};
use node::Node;
use operation::BooleanOperation;
use status::Status;

use crate::{
    curve::NurbsCurve,
    misc::FloatingPoint,
    prelude::{Contains, CurveIntersectionSolverOptions},
    region::{CompoundCurve, Region},
};

mod degeneracies;
pub mod node;
pub mod operation;
pub mod status;
mod vertex;

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
    // type Output = anyhow::Result<Vec<Region<T>>>;
    type Output = anyhow::Result<(Vec<Region<T>>, Vec<Node<T>>)>;
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

    /// Boolean operation for two curves.
    /// Base algorithm reference: Efficient clipping of arbitrary polygons (https://www.inf.usi.ch/hormann/papers/Greiner.1998.ECO.pdf)
    fn boolean(
        &self,
        operation: BooleanOperation,
        other: &'a NurbsCurve<T, Const<3>>,
        option: Self::Option,
    ) -> Self::Output {
        let intersections = find_intersections_without_degeneracies(self, other, option.clone())?
            .into_iter()
            .enumerate()
            .collect_vec();
        // println!("intersections: {}", intersections.len());

        if intersections.is_empty() {
            anyhow::bail!("Todo: no intersections case");
        }

        // create linked list
        let mut a = intersections
            .iter()
            .sorted_by(|(_, i0), (_, i1)| i0.a().1.partial_cmp(&i1.a().1).unwrap())
            .map(|(i, it)| (*i, Rc::new(RefCell::new(Node::new(true, it.a().into())))))
            .collect_vec();

        let mut b = intersections
            .iter()
            .sorted_by(|(_, i0), (_, i1)| i0.b().1.partial_cmp(&i1.b().1).unwrap())
            .map(|(i, it)| (*i, Rc::new(RefCell::new(Node::new(false, it.b().into())))))
            .collect_vec();

        // connect neighbors
        a.iter_mut().for_each(|(index, node)| {
            if let Some((_, neighbor)) = b.iter().find(|(i, _)| i == index) {
                node.borrow_mut().set_neighbor(Rc::downgrade(neighbor));
            }
        });
        b.iter_mut().for_each(|(index, node)| {
            if let Some((_, neighbor)) = a.iter().find(|(i, _)| i == index) {
                node.borrow_mut().set_neighbor(Rc::downgrade(neighbor));
            }
        });

        // remove indices
        let a = a.into_iter().map(|(_, n)| n).collect_vec();
        let b = b.into_iter().map(|(_, n)| n).collect_vec();

        [&a, &b].iter().for_each(|list| {
            list.iter()
                .cycle()
                .take(list.len() + 1)
                .collect_vec()
                .windows(2)
                .for_each(|w| {
                    w[0].borrow_mut().set_next(Rc::downgrade(w[1]));
                    w[1].borrow_mut().set_prev(Rc::downgrade(w[0]));
                });
        });

        let mut a_flag = !other.contains(&self.point_at(self.knots_domain().0), option.clone())?;
        let mut b_flag = !self.contains(&other.point_at(other.knots_domain().0), option.clone())?;
        // println!("a flag: {}, b flag: {}", a_flag, b_flag);

        a.iter().for_each(|list| {
            let mut node = list.borrow_mut();
            *node.status_mut() = if a_flag { Status::Enter } else { Status::Exit };
            a_flag = !a_flag;
        });

        b.iter().for_each(|list| {
            let mut node = list.borrow_mut();
            *node.status_mut() = if b_flag { Status::Enter } else { Status::Exit };
            b_flag = !b_flag;
        });

        match operation {
            BooleanOperation::Union | BooleanOperation::Difference => {
                // invert a status
                a.iter().for_each(|node| {
                    node.borrow_mut().status_mut().invert();
                });
            }
            _ => {}
        }

        if operation == BooleanOperation::Union {
            // invert b status
            b.iter().for_each(|node| {
                node.borrow_mut().status_mut().invert();
            });
        }

        let mut regions = vec![];

        let non_visited = |node: Rc<RefCell<Node<T>>>| -> Option<Rc<RefCell<Node<T>>>> {
            let mut non_visited = node.clone();

            if non_visited.borrow().visited() {
                loop {
                    let next = non_visited.borrow().next()?;
                    non_visited = next;
                    if Rc::ptr_eq(&node, &non_visited) || !non_visited.borrow().visited() {
                        break;
                    }
                }
            }

            if non_visited.borrow().visited() {
                None
            } else {
                Some(non_visited)
            }
        };

        let subject = &a[0];

        /*
        println!(
            "a statues: {:?}",
            a.iter().map(|n| n.borrow().status()).collect_vec()
        );
        println!(
            "a parameters: {:?}",
            a.iter()
                .map(|n| n.borrow().vertex().parameter())
                .collect_vec()
        );
        println!(
            "b statues: {:?}",
            b.iter().map(|n| n.borrow().status()).collect_vec()
        );
        */

        loop {
            match non_visited(subject.clone()) {
                Some(start) => {
                    // println!("start: {:?}", start.borrow().vertex.parameter());
                    let mut nodes = vec![];
                    let mut current = start.clone();
                    loop {
                        if current.borrow().visited() {
                            break;
                        }

                        // let forward = matches!(current.borrow().status(), Status::Enter);
                        current.borrow_mut().visit();
                        nodes.push(current.clone());

                        let node = current.borrow().clone();
                        let next = match node.status() {
                            Status::Enter => node.next(),
                            Status::Exit => node.prev(),
                            Status::None => todo!(),
                        };

                        if let Some(next) = next {
                            next.borrow_mut().visit();
                            nodes.push(next.clone());
                            if let Some(neighbor) = next.borrow().neighbor() {
                                current = neighbor.clone();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    // println!("nodes: {:?}", nodes.len());
                    let mut spans = vec![];
                    for chunk in nodes.chunks(2) {
                        if chunk.len() != 2 {
                            break;
                        }
                        let n0 = chunk[0].borrow();
                        let n1 = chunk[1].borrow();

                        /*
                        println!(
                            "{:?} {:?} -> {:?} {:?}",
                            n0.subject(),
                            n0.status(),
                            n1.subject(),
                            n1.status()
                        );
                        */

                        let params = match (n0.status(), n1.status()) {
                            (Status::Enter, Status::Exit) => {
                                (n0.vertex().parameter(), n1.vertex().parameter())
                            }
                            (Status::Exit, Status::Enter) => {
                                (n1.vertex().parameter(), n0.vertex().parameter())
                            }
                            _ => {
                                // println!("Invalid status");
                                break;
                            }
                        };

                        match (n0.subject(), n1.subject()) {
                            (true, true) => {
                                let trimmed = try_trim(self, params)?;
                                spans.extend(trimmed);
                            }
                            (false, false) => {
                                let trimmed = try_trim(other, params)?;
                                spans.extend(trimmed);
                            }
                            _ => {
                                // println!("subject & clip case");
                            }
                        }
                    }

                    let region = Region::new(CompoundCurve::new(spans), vec![]);
                    regions.push(region);
                }
                None => {
                    break;
                }
            }
        }

        /*
        let length = regions.iter().map(|r| {
            r.exterior().try_length().unwrap()
        }).fold(T::zero(), |a, b| a + b);
        println!("length: {}", length);
        */

        Ok((regions, a.iter().map(|n| n.borrow().clone()).collect_vec()))
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
    let inside = parameters.0 < parameters.1;
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

#[cfg(test)]
mod tests;
