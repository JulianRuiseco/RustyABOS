extern crate nalgebra as na;

use crate::abos_structs::{ABOSImmutable, ABOSInputs, ABOSMutable, INFINITY};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use nalgebra::{DMatrix, DVector, Dim, Dynamic, MatrixMN, U1, U3};

pub fn new_abos(abos_inputs: &ABOSInputs) -> (ABOSImmutable, ABOSMutable) {
    //unwrapping to 1d vector [x1, y1, z1, x2, y2, z2...]
    //real function call would just pass the vector into DMatrix
    let mut vec = Vec::new();
    for point in abos_inputs.points.iter() {
        for ii in point.iter() {
            vec.push(*ii);
        }
    }
    //step 1: make an array with all the points
    let mut xyz_points: MatrixMN<f64, Dynamic, U3> = initialize_dmatrix(&abos_inputs.points);

    //step 2: get Point range information and swap as necessary
    let (x1, x2, y1, y2, z1, z2, xy_swaped) = swap_and_get_ranges(&mut xyz_points);

    let kdtree = initialize_kdtree_from_matrix(&xyz_points);
    //step 3: get Chebyshev distance
    let dmc = get_min_chebyshev_distance_kdi(&xyz_points, &kdtree);
    //step 3: get the grid dimensions
    let (i1, j1, dx, dy) = compute_grid_dimensions(x1, x2, y1, y2, dmc, abos_inputs.filter);

    //step 4: Create empty vectors
    let nb: MatrixMN<usize, Dynamic, Dynamic> = MatrixMN::from_element_generic(
        Dynamic::from_usize(i1 as usize),
        Dynamic::from_usize(j1 as usize),
        0,
    );
    // Pcontainer.matrix::
    let z: DVector<f64> = xyz_points.column(2).clone_owned();

    let res_x = (x2 - x1) / abos_inputs.filter;
    let res_y = (y2 - y1) / abos_inputs.filter;
    let rs = if res_x > res_y { res_x } else { res_y };

    // These items must be calculated with calculated k_max
    let r = 0;
    let l = 0.0;

    let k_u_v: MatrixMN<(usize, usize, usize), Dynamic, Dynamic> = MatrixMN::from_element_generic(
        Dynamic::from_usize(i1 as usize),
        Dynamic::from_usize(j1 as usize),
        (0, 0, 0),
    );

    let mut abos_immutable = ABOSImmutable {
        degree: abos_inputs.degree,
        r,
        l,
        xyz_points, //INPUT all points XYZ
        x1,         //minx
        x2,         //maxx
        y1,         //miny
        y2,         //maxy
        _z1: z1,    //min z
        _z2: z2,    //max z
        _dmc: dmc,  //minimal chebyshev distance
        i1,         //xsize of grid
        j1,         //ysize of grid
        dx,         //size of grid on x
        dy,         //size of grid on y
        z,          //vector if z coordinates XYZ
        nb, // Matrix of nearest points on grid. Containing indexes to nearest point in the XYZ array
        k_u_v, // Grid distance of each grid to the point indexed in nb
        k_max: 0, //maximal element of matrix k
        _rs: rs, // Resolution of map
        _xy_swaped: xy_swaped,
        q_smooth: abos_inputs.q_smooth,
    };

    let p: MatrixMN<f64, Dynamic, Dynamic> = MatrixMN::from_element_generic(
        Dynamic::from_usize(abos_immutable.i1 as usize),
        Dynamic::from_usize(abos_immutable.j1 as usize),
        0.0,
    );
    let dp: MatrixMN<f64, Dynamic, Dynamic> = p.clone_owned();
    let t_smooth: MatrixMN<f64, Dynamic, Dynamic> = p.clone_owned();

    let dz = abos_immutable.z.clone_owned();
    let abos_mutable = ABOSMutable {
        p,
        dp,
        dz,
        t_smooth,
    };

    init_distance_point_matrixes_kdi(&mut abos_immutable, &abos_mutable, &kdtree);
    let (r, l) = compute_rl(abos_inputs.degree, abos_immutable.k_max);
    abos_immutable.r = r;
    abos_immutable.l = l;

    (abos_immutable, abos_mutable)
}

//
pub fn init_distance_point_matrixes_kdi(
    abos_immutable: &mut ABOSImmutable,
    abos_mutable: &ABOSMutable,
    kdtree: &KdTree<f64, usize, [f64; 2]>,
) {
    //iterate through each grid cell position. Set index of point to NB, and grid distance to K
    for (ii, row) in abos_mutable.dp.row_iter().enumerate() {
        for (jj, _col) in row.iter().enumerate() {
            let position = abos_immutable.indexes_to_position(&ii, &jj);
            let kd_search_result = kdtree.nearest(&position, 1, &squared_euclidean).unwrap();
            let closest_point_in_tree_indx = *kd_search_result[0].1;

            let closest_point = abos_immutable.xyz_points.row(closest_point_in_tree_indx);
            let x_distance = f64::round(f64::abs(
                (closest_point[0] - position[0]) / abos_immutable.dx,
            )) as usize;
            let y_distance = f64::round(f64::abs(
                (closest_point[1] - position[1]) / abos_immutable.dy,
            )) as usize;

            unsafe {
                let nb_position = abos_immutable.nb.get_unchecked_mut((ii, jj));
                *nb_position = closest_point_in_tree_indx;

                let k_position = abos_immutable.k_u_v.get_unchecked_mut((ii, jj));
                //*k_position = if x_distance > y_distance { x_distance } else { y_distance };
                let max_cell_dist = if x_distance > y_distance {
                    x_distance
                } else {
                    y_distance
                };
                *k_position = (max_cell_dist, x_distance, y_distance);
                if k_position.0 > (abos_immutable.k_max) {
                    abos_immutable.k_max = k_position.0;
                }
            }
        }
    }

    // println!("{}", abos_immutable.nb);
}

//
// //Make D matrix
// //unwrapping to 1d vector [x1, y1, z1, x2, y2, z2...]
// //real function call would just pass the vector into DMatrix
fn initialize_dmatrix(points: &[Vec<f64>]) -> MatrixMN<f64, Dynamic, U3> {
    let mut vec = Vec::new();
    for point in points.iter() {
        for ii in point.iter() {
            vec.push(*ii);
        }
    }
    let point_count = points.len();
    let dm: DMatrix<f64> = DMatrix::from_iterator(3, point_count, vec.into_iter());
    let fix_dm = dm.transpose();

    //step 1: make an array with all the points
    let xyz_points: MatrixMN<f64, Dynamic, U3> = na::convert(fix_dm);
    xyz_points
}

fn initialize_kdtree_from_matrix(
    xyz_points: &MatrixMN<f64, Dynamic, U3>,
) -> KdTree<f64, usize, [f64; 2]> {
    let mut kdtree = KdTree::new(2);
    // let mut points:Vec<([f64; 2], usize)> = vec![];
    for (i, row) in xyz_points.row_iter().enumerate() {
        let x_position = row[0];
        let y_position = row[1];
        kdtree.add([x_position, y_position], i).unwrap();
    }
    kdtree
}

pub fn get_ranges(points: &MatrixMN<f64, Dynamic, U3>) -> (f64, f64, f64, f64, f64, f64) {
    let x1 = points.column(0).min();
    let x2 = points.column(0).max();
    let y1 = points.column(1).min();
    let y2 = points.column(1).max();
    let z1 = points.column(2).min();
    let z2 = points.column(2).max();

    (x1, x2, y1, y2, z1, z2)
}

pub fn swap_and_get_ranges(
    points: &mut MatrixMN<f64, Dynamic, U3>,
) -> (f64, f64, f64, f64, f64, f64, bool) {
    let (x1, x2, y1, y2, z1, z2) = get_ranges(&points);
    if f64::abs(x1 - x2) < f64::abs(y1 - y2) {
        points.swap_columns(0, 1);
        let (x1, x2, y1, y2, z1, z2) = get_ranges(&points);
        (x1, x2, y1, y2, z1, z2, true)
    } else {
        (x1, x2, y1, y2, z1, z2, false)
    }
}

//
fn get_min_chebyshev_distance_kdi(
    xyz_points: &MatrixMN<f64, Dynamic, U3>,
    kdtree: &KdTree<f64, usize, [f64; 2]>,
) -> f64 {
    let mut min_chebyshev_distance: f64 = INFINITY;

    for row in xyz_points.row_iter() {
        let point = [row[0], row[1]];
        let kd_search_result = kdtree.nearest(&point, 1, &squared_euclidean).unwrap();
        let closest_index = *kd_search_result[0].1;
        let distances: MatrixMN<f64, U1, U3> = row.clone_owned() - xyz_points.row(closest_index);
        let distances = distances.abs();
        let max_xy_distance = if distances[0] > distances[1] {
            distances[0] + 1.0
        } else {
            distances[1] + 1.0
        };

        if max_xy_distance < min_chebyshev_distance {
            min_chebyshev_distance = max_xy_distance;
        }
    }
    min_chebyshev_distance
}

pub fn compute_grid_dimensions(
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    dmc: f64,
    filter: f64,
) -> (i32, i32, f64, f64) {
    //step 1: Always assuming x side is greater

    //step 2: grid size is defined as i0=round  x21/ Dmc 

    let i0: i32 = f64::round((x2 - x1) / dmc) as i32;
    //step 3 find your grid size for your larger dimension set as i1 = i0*k largest possible while less than Filter
    let mut i1: i32 = 0;
    let mut i = 0;
    loop {
        i += 1;
        let potential_val: i32 = i0 * i;
        if filter > potential_val as f64 {
            i1 = potential_val;
        } else {
            break;
        }
    }

    //step 5 find your grid size for your smaller dimension such that it is as close to that of il as possible
    let j1: i32 = f64::round((y2 - y1) / (x2 - x1) * (i1 as f64 - 1.0)) as i32;

    let dx = (x2 - x1) / (i1 - 1) as f64; //include the minus one so matices inclde the max points
    let dy = (y2 - y1) / (j1 - 1) as f64;

    (i1, j1, dx, dy)
}

//
fn compute_rl(degree: i8, k_max: usize) -> (usize, f64) {
    //println!();
    match degree {
        0 => {
            let r = 1;
            let l = 0.7 / ((0.107 * k_max as f64 - 0.714) * k_max as f64);
            (r, l)
        }
        1 => {
            let r = 1;
            let l = 1.0 / ((0.107 * k_max as f64 - 0.714) * k_max as f64);
            (r, l)
        }
        2 => {
            let r = 1;
            let l = 1.0 / (0.0360625 * k_max as f64 + 0.192);
            (r, l)
        }
        3 => {
            let r = 0;
            let l = 0.7 / ((0.107 * k_max as f64 - 0.714) * k_max as f64);
            (r, l)
        }
        _ => (0, 0.0),
    }
}
