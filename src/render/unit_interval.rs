use num_traits::{Num, NumCast};

#[derive(Debug)]
pub struct UnitInterval {
    value: f64,
}

impl UnitInterval {
    pub fn new_linear<V, M>(value: V, max_value: M) -> Self
    where
        V: Num + NumCast,
        M: Num + NumCast,
    {
        assert!(!max_value.is_zero());

        let v = value.to_f64().unwrap() / max_value.to_f64().unwrap();
        UnitInterval { value: v }
    }

    pub fn new_sigmoid_range_abs<V, M, K>(start: V, end: V, max_value: M, k: K) -> Self
    where
        V: Num + NumCast,
        M: Num + NumCast,
        K: Num + NumCast,
    {
        let value = (start.to_f64().unwrap() - end.to_f64().unwrap()).abs();
        UnitInterval::new_sigmoid(value, max_value, k)
    }

    pub fn new_sigmoid<V, M, K>(value: V, max_value: M, k: K) -> Self
    where
        V: Num + NumCast,
        M: Num + NumCast,
        K: Num + NumCast,
    {
        assert!(!max_value.is_zero());

        let value = value.to_f64().unwrap();
        let max_value = max_value.to_f64().unwrap();
        let k = k.to_f64().unwrap();

        let v = 1.0 / (1.0 + (-k * (value / max_value - 0.5)).exp());
        UnitInterval { value: v }
    }

    pub fn scale<M, R>(&self, max_value: M) -> R
    where
        M: Num + NumCast,
        R: Num + NumCast,
    {
        R::from(self.value * max_value.to_f64().unwrap()).unwrap()
    }
}

pub trait NumUnitIntervalExt {
    fn to_unit<M>(&self, max_value: M) -> UnitInterval
    where
        M: Num + NumCast;
    fn to_unit_sigmoid<M, K>(&self, max_value: M, k: K) -> UnitInterval
    where
        M: Num + NumCast,
        K: Num + NumCast;
}

impl<T> NumUnitIntervalExt for T
where
    T: Num + NumCast + Clone,
{
    fn to_unit<M>(&self, max_value: M) -> UnitInterval
    where
        M: Num + NumCast,
    {
        UnitInterval::new_linear(self.clone(), max_value)
    }

    fn to_unit_sigmoid<M, K>(&self, max_value: M, k: K) -> UnitInterval
    where
        M: Num + NumCast,
        K: Num + NumCast,
    {
        UnitInterval::new_sigmoid(self.clone(), max_value, k)
    }
}
