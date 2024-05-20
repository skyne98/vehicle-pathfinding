#[derive(Debug)]
pub struct BitArray {
    bits: Vec<u32>,
    num_bits: usize,
}

impl BitArray {
    // Create a new BitField with a given number of bits
    pub fn new(num_bits: usize) -> Self {
        // Calculate the number of u32 elements needed to hold the bits
        let num_elements = (num_bits + 31) / 32;
        BitArray {
            bits: vec![0; num_elements],
            num_bits,
        }
    }

    // Set a specific bit
    pub fn set_bit(&mut self, position: usize) {
        if position < self.num_bits {
            let element = position / 32;
            let bit = position % 32;
            self.bits[element] |= 1 << bit;
        }
    }

    // Clear a specific bit
    pub fn clear_bit(&mut self, position: usize) {
        if position < self.num_bits {
            let element = position / 32;
            let bit = position % 32;
            self.bits[element] &= !(1 << bit);
        }
    }

    // Toggle a specific bit
    pub fn toggle_bit(&mut self, position: usize) {
        if position < self.num_bits {
            let element = position / 32;
            let bit = position % 32;
            self.bits[element] ^= 1 << bit;
        }
    }

    // Check if a specific bit is set
    pub fn is_bit_set(&self, position: usize) -> bool {
        if position < self.num_bits {
            let element = position / 32;
            let bit = position % 32;
            (self.bits[element] & (1 << bit)) != 0
        } else {
            panic!("Bit index out of range");
        }
    }

    // Set a boolean value for a specific bit
    pub fn set_bool(&mut self, position: usize, value: bool) {
        if value {
            self.set_bit(position);
        } else {
            self.clear_bit(position);
        }
    }

    // Get the boolean value of a specific bit
    pub fn get_bool(&self, position: usize) -> bool {
        self.is_bit_set(position)
    }

    pub fn len(&self) -> usize {
        self.num_bits
    }
}

#[cfg(test)]
mod tests {
    use super::BitArray;

    #[test]
    fn test_set_and_get_bit() {
        let mut bit_array = BitArray::new(64);

        // Initially, all bits should be clear
        assert!(!bit_array.get_bool(0));
        assert!(!bit_array.get_bool(63));

        // Set bit at position 3
        bit_array.set_bool(3, true);
        assert!(bit_array.get_bool(3));

        // Clear bit at position 3
        bit_array.set_bool(3, false);
        assert!(!bit_array.get_bool(3));

        // Set bit at position 5
        bit_array.set_bool(5, true);
        assert!(bit_array.get_bool(5));
    }

    #[test]
    fn test_toggle_bit() {
        let mut bit_array = BitArray::new(64);

        // Toggle bit at position 2
        bit_array.toggle_bit(2);
        assert!(bit_array.get_bool(2));

        // Toggle bit at position 2 again
        bit_array.toggle_bit(2);
        assert!(!bit_array.get_bool(2));
    }

    #[test]
    fn test_out_of_range() {
        let bit_array = BitArray::new(64);

        // Trying to access a bit out of range should panic
        assert!(std::panic::catch_unwind(|| bit_array.get_bool(64)).is_err());
    }
}
