use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, Table, Value},
    plonk::{ConstraintSystem, Error, TableColumn},
};

/// A look up table of values of RANGE length
/// e.g RANGE = 8, values =[0..255]
/// this table is tagged by an index k, where k is the number of bits of the value in the 'value' column.
///
#[derive(Debug, Clone)]
pub(super) struct RangeTableConfig<F: FieldExt, const NUM_BITS: usize, const RANGE: usize> {
    pub(super) num_bits: TableColumn,
    pub(super) value: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: FieldExt, const NUM_BITS: usize, const RANGE: usize> RangeTableConfig<F, NUM_BITS, RANGE> {
    pub(super) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        assert_eq!(1 << NUM_BITS, RANGE);
        let num_bits = meta.lookup_table_column();
        let value: TableColumn = meta.lookup_table_column();

        Self { num_bits, value, _marker: PhantomData }
    }

    pub(super) fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "load range check table",
            |mut table| {
                let mut offset = 0;

                table.assign_cell(
                    || "assign num_bits = 1",
                    self.num_bits,
                    offset,
                    || Value::known(F::one()),
                )?;

                table.assign_cell(
                    || "assign value = 0",
                    self.value,
                    offset,
                    || Value::known(F::zero()),
                )?;

                offset += 1;

                for num_bits in 1..=NUM_BITS {
                    for value in 1 << (num_bits - 1)..(1 << num_bits) {
                        table.assign_cell(
                            || "assign num_bits",
                            self.num_bits,
                            offset,
                            || Value::known(F::from(num_bits as u64)),
                        )?;

                        table.assign_cell(
                            || "assign value",
                            self.value,
                            offset,
                            || Value::known(F::from(value as u64)),
                        )?;

                        offset += 1;
                    }
                }

                Ok(())
            },
        )
    }
}
