use std::collections::HashMap;


pub mod cli;

pub struct BurrowDB{
    data:HashMap<String,String>,
}

impl BurrowDB{
    //Constructor
    pub fn new()->Self{
        BurrowDB {
             data: HashMap::new(),
        }
    }

    //Put function for craete and update

    pub fn put(&mut self,key:String,value:String){
        self.data.insert(key, value);
    }

    //Get funtion for reading the value
    //Read value by refrence pointing to the key, instead of searching through it
    pub fn get(&self, key: &str)-> Option<&str>{
        self.data.get(key).map(|s| s.as_str())
    }






}