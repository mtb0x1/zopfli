use libc::{size_t, c_double, c_uchar, c_int};

use deflate::calculate_block_size_auto_type;
use lz77::{ZopfliLZ77Store, Lz77Store, lz77_store_from_c};
use symbols::{ZOPFLI_LARGE_FLOAT};
use zopfli::ZopfliOptions;

/// Finds minimum of function f(i) where is is of type size_t, f(i) is of type
/// double, i is in range start-end (excluding end).
/// Outputs the minimum value in *smallest and returns the index of this value.
#[no_mangle]
#[allow(non_snake_case)]
pub extern fn FindMinimum(f: fn(i: size_t, context: &SplitCostContext) -> c_double, context: &SplitCostContext, start: size_t, end: size_t) -> (size_t, c_double) {
    let mut start = start;
    let mut end = end;
    if end - start < 1024 {
        let mut best = ZOPFLI_LARGE_FLOAT;
        let mut result = start;
        for i in start..end {
            let v = f(i, context);
            if v < best {
                best = v;
                result = i;
            }
        }
        (result, best)
    } else {
        /* Try to find minimum faster by recursively checking multiple points. */
        let num = 9;  /* Good value: 9. ?!?!?!?! */
        let mut p = vec![0; num];
        let mut vp = vec![0.0; num];
        let mut besti;
        let mut best;
        let mut lastbest = ZOPFLI_LARGE_FLOAT;
        let mut pos = start;

        loop {
            if end - start <= num {
                break;
            }

            for i in 0..num {
                p[i] = start + (i + 1) * ((end - start) / (num + 1));
                vp[i] = f(p[i], context);
            }

            besti = 0;
            best = vp[0];

            for i in 1..num {
                if vp[i] < best {
                  best = vp[i];
                  besti = i;
                }
            }
            if best > lastbest {
                break;
            }

            start = if besti == 0 { start } else { p[besti - 1] };
            end = if besti == num - 1 { end } else { p[besti + 1] };

            pos = p[besti];
            lastbest = best;
        }
        (pos, lastbest)
    }
}

/// Returns estimated cost of a block in bits.  It includes the size to encode the
/// tree and the size to encode all literal, length and distance symbols and their
/// extra bits.
///
/// litlens: lz77 lit/lengths
/// dists: ll77 distances
/// lstart: start of block
/// lend: end of block (not inclusive)
#[no_mangle]
#[allow(non_snake_case)]
pub extern fn EstimateCost(lz77: &Lz77Store, lstart: size_t, lend: size_t) -> c_double {
    calculate_block_size_auto_type(lz77, lstart, lend)
}

/// Gets the cost which is the sum of the cost of the left and the right section
/// of the data.
/// type: FindMinimumFun
#[allow(non_snake_case)]
pub fn SplitCost(i: size_t, c: &SplitCostContext) -> c_double {
    EstimateCost(c.lz77, c.start, i) + EstimateCost(c.lz77, i, c.end)
}

pub struct SplitCostContext<'a> {
    lz77: &'a Lz77Store,
    start: size_t,
    end: size_t,
}

/// Finds next block to try to split, the largest of the available ones.
/// The largest is chosen to make sure that if only a limited amount of blocks is
/// requested, their sizes are spread evenly.
/// lz77size: the size of the LL77 data, which is the size of the done array here.
/// done: array indicating which blocks starting at that position are no longer
///     splittable (splitting them increases rather than decreases cost).
/// splitpoints: the splitpoints found so far.
/// npoints: the amount of splitpoints found so far.
/// lstart: output variable, giving start of block.
/// lend: output variable, giving end of block.
/// returns 1 if a block was found, 0 if no block found (all are done).
#[no_mangle]
#[allow(non_snake_case)]
pub extern fn FindLargestSplittableBlock(lz77size: size_t, done: *const c_uchar, splitpoints: *const size_t, npoints: size_t, lstart: size_t, lend: size_t) -> (c_int, size_t, size_t) {
    let mut longest = 0;
    let mut found = 0;
    let mut lstart = lstart;
    let mut lend = lend;

    for i in 0..(npoints + 1) {
        let start = if i == 0 { 0 } else { unsafe { *splitpoints.offset(i as isize - 1) } };
        let end = if i == npoints { lz77size - 1 } else { unsafe { *splitpoints.offset(i as isize) } };
        if unsafe { *done.offset(start as isize) } == 0 && end - start > longest {
            lstart = start;
            lend = end;
            found = 1;
            longest = end - start;
        }
    }

    (found, lstart, lend)
}

/// Prints the block split points as decimal and hex values in the terminal.
#[no_mangle]
#[allow(non_snake_case)]
pub extern fn PrintBlockSplitPoints(lz77: &Lz77Store, lz77splitpoints: *const size_t, nlz77points: size_t) {
    let mut splitpoints = Vec::with_capacity(nlz77points);

    /* The input is given as lz77 indices, but we want to see the uncompressed
    index values. */
    let mut pos = 0;
    if nlz77points > 0 {
        for i in 0..lz77.size() {
            let length = if lz77.dists[i] == 0 {
                1
            } else {
                lz77.litlens[i]
            };
            if unsafe { *lz77splitpoints.offset(splitpoints.len() as isize) } == i {
                splitpoints.push(pos);
                if splitpoints.len() == nlz77points {
                    break;
                }
            }
            pos += length;
        }
    }
    assert!(splitpoints.len() == nlz77points);

    println!("block split points: {} (hex: {})", splitpoints.iter().map(|&sp| format!("{}", sp)).collect::<Vec<_>>().join(" "), splitpoints.iter().map(|&sp| format!("{:x}", sp)).collect::<Vec<_>>().join(" "));
}

#[link(name = "zopfli")]
extern {
    fn AddSorted(value: size_t, out: *mut *mut size_t, outsize: *mut size_t);
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern fn ZopfliBlockSplitLZ77(options_ptr: *const ZopfliOptions, lz77_ptr: *const ZopfliLZ77Store, maxblocks: size_t, splitpoints: *mut *mut size_t, npoints: *mut size_t) {
    let options = unsafe {
        assert!(!options_ptr.is_null());
        &*options_ptr
    };
    let lz77_still_pointer = lz77_store_from_c(lz77_ptr);
    let lz77 = unsafe { &*lz77_still_pointer };

    if lz77.size() < 10 {
        return;  /* This code fails on tiny files. */
    }

    let mut llpos;
    let mut numblocks = 1;
    let mut splitcost: c_double;
    let mut origcost;
    let mut done = vec![0; lz77.size()];
    let mut lstart = 0;
    let mut lend = lz77.size();

    loop {
        if maxblocks > 0 && numblocks >= maxblocks {
          break;
        }
        let c = SplitCostContext {
            lz77: lz77,
            start: lstart,
            end: lend,
        };

        assert!(lstart < lend);
        let find_minimum_result = FindMinimum(SplitCost, &c, lstart + 1, lend);
        llpos = find_minimum_result.0;
        splitcost = find_minimum_result.1;

        assert!(llpos > lstart);
        assert!(llpos < lend);

        origcost = EstimateCost(lz77, lstart, lend);

        if splitcost > origcost || llpos == lstart + 1 || llpos == lend {
            done[lstart] = 1;
        } else {
            unsafe { AddSorted(llpos, splitpoints, npoints) };
            numblocks += 1;
        }

        let find_block_results = FindLargestSplittableBlock(lz77.size(), done.as_ptr(), unsafe { *splitpoints }, unsafe { *npoints }, lstart, lend);
        lstart = find_block_results.1;
        lend = find_block_results.2;

        if find_block_results.0 == 0 {
            break;  /* No further split will probably reduce compression. */
        }

        if lend - lstart < 10 {
            break;
        }
    }

    if options.verbose > 0 {
        PrintBlockSplitPoints(lz77, unsafe { *splitpoints }, unsafe { *npoints });
    }
}
