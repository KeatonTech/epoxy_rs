use super::{ReactiveValue, ReadonlyReactiveValue};
use std::sync::Arc;

impl<T: 'static + Send + Sync> dyn ReactiveValue<T> {
    /// Returns a ReactiveValue that only changes when the content of the original ReactiveValue
    /// passes a test (specified by `filter_function`). The output ReactiveValue will be equal
    /// to `default_value` until the original ReactiveValue passes the test.
    ///
    /// *Note:* This function is roughly equivalent to the `filter` operator on Streams, but
    /// is renamed to clarify its behavior.
    ///
    /// # Examples
    /// ```
    /// use epoxy_streams::ReactiveValue;
    ///
    /// let original = ReactiveValue::new(1_i32);
    /// let only_even = ReactiveValue::sanitize(&original, |val| val % 2 == 0, 0);
    /// assert_eq!(*only_even.get(), 0);
    ///
    /// original.set(2);
    /// assert_eq!(*only_even.get(), 2);
    ///
    /// original.set(3);
    /// assert_eq!(*only_even.get(), 2);
    ///
    /// original.set(4);
    /// assert_eq!(*only_even.get(), 4);
    /// ```
    pub fn sanitize<F>(
        value: &dyn ReactiveValue<T>,
        filter_function: F,
        default_value: T,
    ) -> ReadonlyReactiveValue<T>
    where
        F: Fn(&T) -> bool,
        F: Send,
        F: Sync,
        F: 'static,
    {
        value
            .as_stream()
            .filter(filter_function)
            .to_reactive_value_with_default(default_value)
    }

    /// Returns a ReactiveValue that reverts to a given value when the filter test fails.
    ///
    /// # Examples
    /// ```
    /// use epoxy_streams::ReactiveValue;
    ///
    /// let original = ReactiveValue::new(1_i32);
    /// let only_even = ReactiveValue::sanitize(&original, |val| val % 2 == 0, 0);
    /// assert_eq!(*only_even.get(), 0);
    ///
    /// original.set(2);
    /// assert_eq!(*only_even.get(), 2);
    ///
    /// original.set(3);
    /// assert_eq!(*only_even.get(), 2);
    ///
    /// original.set(4);
    /// assert_eq!(*only_even.get(), 4);
    /// ```
    pub fn fallback<F>(
        value: &dyn ReactiveValue<T>,
        filter_function: F,
        fallback_value: T,
    ) -> ReadonlyReactiveValue<T>
    where
        F: Fn(&T) -> bool,
        F: Send,
        F: Sync,
        F: 'static,
    {
        let fallback_value_rc = Arc::new(fallback_value);
        let inner_rc = fallback_value_rc.clone();
        value
            .as_stream()
            .map_rc(move |val| {
                if filter_function(&*val) {
                    val
                } else {
                    inner_rc.clone()
                }
            })
            .to_reactive_value_with_default_rc(fallback_value_rc)
    }

    /// Returns a ReactiveValue whose content is the result of running the content of the original
    /// ReactiveValue through a mapping function.
    ///
    /// # Examples
    ///
    /// ```
    /// use epoxy_streams::ReactiveValue;
    ///
    /// let original = ReactiveValue::new("Bread");
    /// let thing_that_is_cool = ReactiveValue::map(&original, |str| {
    ///     format!("{} is cool", str)
    /// });
    /// assert_eq!(*thing_that_is_cool.get(), "Bread is cool");
    ///
    /// original.set("Cheese");
    /// assert_eq!(*thing_that_is_cool.get(), "Cheese is cool");
    ///
    /// ```
    pub fn map<U, F>(value: &dyn ReactiveValue<T>, map_function: F) -> ReadonlyReactiveValue<U>
    where
        U: 'static,
        U: Send,
        U: Sync,
        F: Fn(&T) -> U,
        F: Send,
        F: Sync,
        F: 'static,
    {
        let default = map_function(&*value.get());
        value
            .as_stream()
            .map(map_function)
            .to_reactive_value_with_default(default)
    }
}
