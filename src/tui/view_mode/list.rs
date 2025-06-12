use std::rc::Rc;

pub trait ListFilter<T> {
    fn filter(&self, item: &T) -> bool;
}

pub trait ListSorter<T> {
    fn compare(&self, a: &T, b: &T) -> std::cmp::Ordering;
}

pub struct ListViewMode<T> {
    pub filters: Vec<Rc<dyn ListFilter<T>>>,
    pub sorters: Vec<Rc<dyn ListSorter<T>>>,
}

impl<T> ListViewMode<T> {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            sorters: Vec::new(),
        }
    }

    pub fn apply<'a>(&self, items: &'a [T]) -> Vec<&'a T> {
        let mut filtered: Vec<&T> = items
            .iter()
            .filter(|item| self.filters.iter().all(|f| f.filter(item)))
            .collect();
        for sorter in &self.sorters {
            filtered.sort_by(|a, b| sorter.compare(a, b));
        }
        filtered
    }

    pub fn add_filter(&mut self, filter: Rc<dyn ListFilter<T>>) {
        self.filters.push(filter);
    }

    pub fn add_sorter(&mut self, sorter: Rc<dyn ListSorter<T>>) {
        self.sorters.push(sorter);
    }
} 