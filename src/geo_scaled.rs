use geo::{BooleanOps, Coord, GeoNum, MultiPolygon, Polygon};

trait Scaling {
    type RetType;
    fn to_integer_polygon(&self, scale: f32) -> Self::RetType;
    fn from_integer_polygon(polygon: &Self::RetType, scale: f32) -> Self;
}

impl Scaling for Polygon<f32> {
    type RetType = Polygon<f32>;
    fn to_integer_polygon(&self, scale: f32) -> Self::RetType {
        let exterior = self
            .exterior()
            .coords()
            .map(|c| Coord {
                x: (c.x * scale).round(),
                y: (c.y * scale).round(),
            })
            .collect();
        let interiors = self
            .interiors()
            .iter()
            .map(|interior| {
                interior
                    .into_iter()
                    .map(|c| Coord {
                        x: (c.x * scale).round(),
                        y: (c.y * scale).round(),
                    })
                    .collect()
            })
            .collect();

        Polygon::new(exterior, interiors)
    }
    fn from_integer_polygon(polygon: &Self::RetType, scale: f32) -> Self {
        let exterior = polygon
            .exterior()
            .coords()
            .map(|c| Coord {
                x: c.x / scale,
                y: c.y / scale,
            })
            .collect();
        let interiors = polygon
            .interiors()
            .iter()
            .map(|interior| {
                interior
                    .into_iter()
                    .map(|c| Coord {
                        x: c.x / scale,
                        y: c.y / scale,
                    })
                    .collect()
            })
            .collect();

        Polygon::new(exterior, interiors)
    }
}

impl Scaling for MultiPolygon<f32> {
    type RetType = MultiPolygon<f32>;
    fn to_integer_polygon(&self, scale: f32) -> Self::RetType {
        self.iter().map(|p| p.to_integer_polygon(scale)).collect()
    }
    fn from_integer_polygon(polygon: &Self::RetType, scale: f32) -> Self {
        polygon
            .iter()
            .map(|p| Polygon::<f32>::from_integer_polygon(p, scale))
            .collect()
    }
}

pub trait ScaledBooleanOps {
    type Scalar: GeoNum;
    fn scaled_intersection(&self, other: &Self, scale: f32) -> MultiPolygon<Self::Scalar>;
    fn scaled_union(&self, other: &Self, scale: f32) -> MultiPolygon<Self::Scalar>;
    fn xor(&self, other: &Self) -> MultiPolygon<Self::Scalar>;
    fn difference(&self, other: &Self) -> MultiPolygon<Self::Scalar>;
}

impl ScaledBooleanOps for MultiPolygon<f32> {
    type Scalar = f32;
    fn scaled_intersection(&self, other: &Self, scale: f32) -> MultiPolygon<Self::Scalar> {
        let p = self.to_integer_polygon(scale);
        let q = other.to_integer_polygon(scale);
        MultiPolygon::<f32>::from_integer_polygon(
            &<MultiPolygon<f32> as BooleanOps>::intersection(&p, &q),
            scale,
        )
    }
    fn scaled_union(&self, other: &Self, scale: f32) -> MultiPolygon<Self::Scalar> {
        let p = self.to_integer_polygon(scale);
        let q = other.to_integer_polygon(scale);
        MultiPolygon::<f32>::from_integer_polygon(
            &<MultiPolygon<f32> as BooleanOps>::union(&p, &q),
            scale,
        )
    }
    fn xor(&self, _other: &Self) -> MultiPolygon<Self::Scalar> {
        unimplemented!()
    }
    fn difference(&self, _other: &Self) -> MultiPolygon<Self::Scalar> {
        unimplemented!()
    }
}
