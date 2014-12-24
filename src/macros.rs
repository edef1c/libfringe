macro_rules! __into_fields {
  ($x:ident { $field:ident } <- $iter:ident) => {
    match $iter.next() {
      Some(value) => {
        $x.$field = value;
        Some($iter)
      }
      None => None
    }
  };
  ($x:ident { $field:ident, $($fields_rest:ident),* } <- $iter:ident) => {
    match $iter.next() {
      Some(value) => {
        $x.$field = value;
        __into_fields!($x { $($fields_rest),* } <- $iter)
      }
      None => None
    }
  };
}

macro_rules! into_fields {
  ($x:ident { $($fields_rest:ident),* } <- $iter:expr) => {
    {
      let mut iter = $iter;
      __into_fields!($x { $($fields_rest),* } <- iter)
    }
  }
}
