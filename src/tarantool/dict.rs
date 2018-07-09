use std::sync::RwLock;
use std::collections::HashMap;
use std::io;


lazy_static! {
    pub static ref SPACE_DICTIONARY: RwLock<HashMap<String,u32>> = RwLock::new(HashMap::new());
    pub static ref SPACE_INDEX_DICTIONARY: RwLock<HashMap<u32,HashMap<String,u32>>> = RwLock::new(HashMap::new());
}

pub fn clear_dictionaries()  {
    SPACE_DICTIONARY.write().unwrap().clear();
    SPACE_INDEX_DICTIONARY.write().unwrap().clear();
}

pub fn add_space_dict_entry(space_id:u32, name:String) -> io::Result<()> {
    SPACE_DICTIONARY.write().unwrap().insert(name, space_id);
    Ok(())
}

pub fn add_space_index_dict_entry(space_id:u32, index_id:u32, name:String) -> io::Result<()> {
    let mut data = SPACE_INDEX_DICTIONARY.write().unwrap();
    data.entry(space_id).or_insert_with(HashMap::new).insert(name, index_id);
    Ok(())
}

pub fn search_space_id(name:&str) -> Option<u32> {
    return SPACE_DICTIONARY.read().unwrap().get(name).map(|v|*v)
}

pub fn search_index_id(space_id:u32, index_name:&str) -> Option<u32> {
    return SPACE_INDEX_DICTIONARY.read().unwrap()
        .get(&space_id)
        .and_then(|v|v.get(index_name))
        .map(|v|*v)
}

