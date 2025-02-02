use std::marker::PhantomData;

use halo2_base::{halo2_proofs::{
    arithmetic::FieldExt,
    circuit::*,
    plonk::*,
    pasta::Fp, 
    dev::MockProver, 
    poly::Rotation,
}};

#[derive(Debug, Clone)]
struct Acell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FiboConfig{
    pub advice: [Column<Advice>; 3],
    pub selector: Selector,
    pub instance: Column<Instance>
}

struct FiboChip<F: FieldExt> {
    config: FiboConfig, 
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FiboChip<F> {
    fn construct(config: FiboConfig) -> Self {
        Self {config, _marker: PhantomData}
    }

    fn configure(meta: &mut ConstraintSystem<F>,
                advice: [Column<Advice>; 3],
                instance: Column<Instance>,
            ) -> FiboConfig {
        let col_a = advice[0];
        let col_b = advice[1];
        let col_c = advice[2];

        let selector = meta.selector();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta: &mut VirtualCells<F>| {

            let s= meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());
            vec![s * ( a + b - c)]
        });

        FiboConfig { 
            advice:[col_a, col_b, col_c], 
            selector,
            instance
        }
    }

    fn assign_first_row(&self, mut layouter: impl Layouter<F>, a: Option<F>, b: Option<F>)
    -> Result<(Acell<F>, Acell<F>, Acell<F>), Error> {
        layouter.assign_region(||"first row", 
        |mut region| {
            self.config.selector.enable(&mut region, 0)?;
            
            let a_cell = region.assign_advice(
                ||"a",
                self.config.advice[0], 
                0, 
                ||a.ok_or(Error::Synthesis)
            ).map(Acell)?;

            let b_cell = region.assign_advice(
                ||"b",
                self.config.advice[1], 
                0, 
                ||a.ok_or(Error::Synthesis)
            ).map(Acell)?;

            let c_val = a.and_then(|a| b.map(|b| a + b));
        
            let c_cell = region.assign_advice(
                ||"c",
                self.config.advice[2], 
                0, 
                ||c_val.ok_or(Error::Synthesis)
            ).map(Acell)?;

            Ok((a_cell, b_cell, c_cell))
        })
    }

    fn assign_row(&self, mut layouter: impl Layouter<F>,  prev_b: &Acell<F>, prev_c: &Acell<F>)
        -> Result<Acell<F>, Error> {
            layouter.assign_region(
                ||"next row",
                |mut region: Region<F>| {
                    self.config.selector.enable(&mut region, 0)?;


                    //permutation trick
                    prev_b.0.copy_advice(||"a", &mut region, self.config.advice[0], 0)?;
                    prev_c.0.copy_advice(||"b", &mut region, self.config.advice[1], 0)?;
                
                    let c_val = prev_b.0.value().and_then(
                        |b| {
                            prev_c.0.value().map(|c| *b + *c)
                        });

                    let c_cell = region.assign_advice(
                        ||"c", 
                        self.config.advice[2],
                        0,
                        ||c_val.ok_or(Error::Synthesis),
                    ).map(Acell)?;

                Ok(c_cell)
            })

        }

        pub fn expose_public(&self,
            mut layouter: impl Layouter<F>,
            cell: &Acell<F>,
            row: usize) -> Result<(), Error>{
                layouter.constrain_instance(cell.0.cell(), 
                self.config.instance,
                row)
            }
}

#[derive(Default)]
struct MyCircuit<F> {
    pub a: Option<F>,
    pub b: Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;
    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let instance = meta.instance_column();
        FiboChip::configure(meta, [col_a, col_b, col_c], instance)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip =  FiboChip::construct(config);
        let (prev_a, mut prev_b, mut prev_c) = chip.assign_first_row(
            layouter.namespace(||"first row"),
            self.a, self.b,
        )?;

        // Define the copy constraint from the instance column to our relevant advice cell
        chip.expose_public(layouter.namespace(||"private a"), &prev_a, 0)?;
        chip.expose_public(layouter.namespace(||"private b"), &prev_b, 1)?;

        for _i in 3..10{
            let c_cell = chip.assign_row(
                layouter.namespace(||"next row"),
                &prev_b,
                &prev_c,
            )?;
            prev_b = prev_c;
            prev_c = c_cell;
        }

        chip.expose_public(layouter.namespace(||"out"), &prev_c, 2)?;

        Ok(())
    }
}


fn main() {
    let k = 4;

    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);

    let circuit = MyCircuit{
        a: Some(a),
        b: Some(b),
    };

    let public_input = vec![a, b, out];

    let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();

    prover.assert_satisfied();


    println!("Hello, world!");
}
