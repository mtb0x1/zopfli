use std::cmp::Ordering;
// use std::{mem};

use typed_arena::Arena;

// Bounded package merge algorithm, based on the paper
// "A Fast and Space-Economical Algorithm for Length-Limited Coding
// Jyrki Katajainen, Alistair Moffat, Andrew Turpin".

#[derive(Debug)]
struct Node<'a> {
    weight: usize,
    count: usize,
    tail: Option<&'a mut Node<'a>>,
}

#[derive(Debug)]
struct Leaf {
    weight: usize,
    count: usize,
}
impl PartialEq for Leaf {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight
    }
}
impl Eq for Leaf { }
impl Ord for Leaf {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}
impl PartialOrd for Leaf {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
struct List<'a> {
    lookahead0: &'a Node<'a>,
    lookahead1: &'a Node<'a>,
}

/// Calculates the bitlengths for the Huffman tree, based on the counts of each
/// symbol.
pub fn length_limited_code_lengths(frequencies: &[usize], max_bits: usize) -> Vec<u32> {
    let num_freqs = frequencies.len();
    let mut bit_lengths = vec![0; num_freqs];

    // Count used symbols and place them in the leaves.
    let mut leaves: Vec<_> = frequencies.iter()
        .enumerate()
        .filter(|&(_, &freq)| freq != 0)
        .map(|(i, &freq)| Leaf { weight: freq, count: i })
        .collect();

    let num_symbols = leaves.len();

    // Short circuit some special cases

    // TODO:
    // if ((1 << maxbits) < numsymbols) {
    //   free(leaves);
    //   return 1;  /* Error, too few maxbits to represent symbols. */
    // }

    if num_symbols == 0 {
        // There are no non-zero frequencies.
        return bit_lengths;
    }
    if num_symbols == 1 {
        bit_lengths[leaves[0].count] = 1;
        return bit_lengths;
    }
    if num_symbols == 2 {
        bit_lengths[leaves[0].count] = 1;
        bit_lengths[leaves[1].count] = 1;
        return bit_lengths;
    }

    // Sort the leaves from least frequent to most frequent.
    leaves.sort();

    let max_bits = if num_symbols - 1 < max_bits {
        num_symbols - 1
    } else {
        max_bits
    };

    let arena_capacity = max_bits * 2 * num_symbols;
    let node_arena: Arena<Node> = Arena::with_capacity(arena_capacity);

    let node0 = node_arena.alloc(Node {
        weight: leaves[0].weight,
        count: 1,
        tail: None,
    });

    let node1 = node_arena.alloc(Node {
        weight: leaves[1].weight,
        count: 2,
        tail: None,
    });

    let mut lists: Vec<List> = vec![
        List {
            lookahead0: node0,
            lookahead1: node1,
        };
        max_bits
    ];

    // let max_num_leaves = 2 * num_symbols - 2;
    // let mut lists = vec![
    //     List {
    //         lookahead0: Node::new(leaves[0].weight, 1, max_num_leaves),
    //         lookahead1: Node::new(leaves[1].weight, 2, max_num_leaves),
    //         next_leaf_index: 2,
    //     };
    //     max_bits
    // ];
    //
    // // In the last list, 2 * numsymbols - 2 active chains need to be created. Two
    // // are already created in the initialization. Each boundary_pm run creates one.
    // let num_boundary_pm_runs = max_num_leaves - 2;
    // for _ in 0..num_boundary_pm_runs {
    //     boundary_pm_toplevel(&mut lists[..], &leaves);
    // }
    //
    // let mut a = lists.pop().unwrap().lookahead1.leaf_counts.into_iter().rev().peekable();
    //
    // let mut bitlength_value = 1;
    // while let Some(leaf_count) = a.next() {
    //     let next_count = *a.peek().unwrap_or(&0);
    //     for leaf in &leaves[next_count..leaf_count] {
    //         bit_lengths[leaf.count] = bitlength_value;
    //     }
    //     bitlength_value += 1;
    // }

    bit_lengths
}

// fn lowest_list(lists: &mut [List], leaves: &[Leaf]) {
//     // We're in the lowest list, just add another leaf to the lookaheads
//     // There will always be more leaves to be added on level 0 so this is safe.
//     let mut current_list = &mut lists[0];
//     let next_leaf = &leaves[current_list.next_leaf_index];
//     current_list.lookahead1.weight = next_leaf.weight;
//
//     current_list.lookahead1.leaf_counts[0] = current_list.lookahead0.leaf_counts.last().unwrap() + 1;
//     current_list.next_leaf_index += 1;
// }

// fn next_leaf(lists: &mut [List], leaves: &[Leaf], current_list_index: usize) {
//     let mut current_list = &mut lists[current_list_index];
//
//     // The next leaf goes next; counting itself makes the leaf_count increase by one.
//     current_list.lookahead1.weight = leaves[current_list.next_leaf_index].weight;
//     current_list.lookahead1.leaf_counts.clear();
//     current_list.lookahead1.leaf_counts.extend_from_slice(&current_list.lookahead0.leaf_counts);
//     let last_index = current_list.lookahead1.leaf_counts.len() - 1;
//     current_list.lookahead1.leaf_counts[last_index] += 1;
//     current_list.next_leaf_index += 1;
// }

// fn next_tree(weight_sum: usize, lists: &mut [List], leaves: &[Leaf], current_list_index: usize) {
//     {
//         let (head, tail) = lists.split_at_mut(current_list_index);
//         let prev_list = head.last_mut().unwrap();
//         let current_list = tail.first_mut().unwrap();
//
//         let previous_list_leaf_counts = &prev_list.lookahead1.leaf_counts;
//
//         // Make a tree from the lookaheads from the previous list; that goes next.
//         // This is not a leaf node, so the leaf count stays the same.
//         current_list.lookahead1.weight = weight_sum;
//         current_list.lookahead1.leaf_counts.clear();
//
//         current_list.lookahead1.leaf_counts.extend_from_slice(previous_list_leaf_counts);
//         current_list.lookahead1.leaf_counts.push(*current_list.lookahead0.leaf_counts.last().unwrap());
//     }
//
//     // The previous list needs two new lookahead nodes.
//     boundary_pm(lists, leaves, current_list_index - 1);
//     boundary_pm(lists, leaves, current_list_index - 1);
// }

// fn boundary_pm_toplevel(lists: &mut [List], leaves: &[Leaf]) {
//     let last_index = lists.len() - 1;
//     boundary_pm(lists, leaves, last_index);
// }

// fn boundary_pm(lists: &mut [List], leaves: &[Leaf], current_list_index: usize) {
//     let next_leaf_index = lists[current_list_index].next_leaf_index;
//     let num_symbols = leaves.len();
//
//     if current_list_index == 0 && next_leaf_index == num_symbols {
//         // We've added all the leaves to the lowest list, so we're done here
//         return;
//     }
//
//     mem::swap(&mut lists[current_list_index].lookahead0, &mut lists[current_list_index].lookahead1);
//
//     if current_list_index == 0 {
//         lowest_list(lists, leaves);
//     } else {
//         // We're at a list other than the lowest list.
//         let weight_sum = {
//             let previous_list = &lists[current_list_index - 1];
//             previous_list.lookahead0.weight + previous_list.lookahead1.weight
//         };
//
//         if next_leaf_index < num_symbols && weight_sum > leaves[next_leaf_index].weight {
//             next_leaf(lists, leaves, current_list_index);
//         } else {
//             next_tree(weight_sum, lists, leaves, current_list_index);
//         }
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_paper_3() {
        let input = [1, 1, 5, 7, 10, 14];
        let output = length_limited_code_lengths(&input, 3);
        let answer = vec![3, 3, 3, 3, 2, 2];
        assert_eq!(output, answer);
    }

    #[test]
    fn test_from_paper_4() {
        let input = [1, 1, 5, 7, 10, 14];
        let output = length_limited_code_lengths(&input, 4);
        let answer = vec![4, 4, 3, 2, 2, 2];
        assert_eq!(output, answer);
    }

    #[test]
    fn max_bits_7() {
        let input = [252, 0, 1, 6, 9, 10, 6, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let output = length_limited_code_lengths(&input, 7);
        let answer = vec![1, 0, 6, 4, 3, 3, 3, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(output, answer);
    }

    #[test]
    fn max_bits_15() {
        let input = [0, 0, 0, 0, 0, 0, 18, 0, 6, 0, 12, 2, 14, 9, 27, 15, 23, 15, 17, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let output = length_limited_code_lengths(&input, 15);
        let answer = vec! [0, 0, 0, 0, 0, 0, 3, 0, 5, 0, 4, 6, 4, 4, 3, 4, 3, 3, 3, 4, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(output, answer);
    }

    #[test]
    fn no_frequencies() {
        let input = [0, 0, 0, 0, 0];
        let output = length_limited_code_lengths(&input, 7);
        let answer = vec![0, 0, 0, 0, 0];
        assert_eq!(output, answer);
    }

    #[test]
    fn only_one_frequency() {
        let input = [0, 10, 0];
        let output = length_limited_code_lengths(&input, 7);
        let answer = vec![0, 1, 0];
        assert_eq!(output, answer);
    }

    #[test]
    fn only_two_frequencies() {
        let input = [0, 0, 0, 0, 252, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let output = length_limited_code_lengths(&input, 7);
        let answer = [0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(output, answer);
    }
}
