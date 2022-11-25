use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Mutex, Arc};
use std::time::Instant;
use serde::{Serialize, Deserialize};
use serde_json::{Value, Map};
use std::thread;
use std::collections::{HashMap};

//Used for keeping track of when each value was updated.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ValueAndTimestamp {
    name: String,
    value: String,
    timestamp: i64
}

//Structure to parse the json easily with serde.
#[derive(Serialize, Deserialize, Debug)]
struct UpdateRequest {
    user: String,
    timestamp: i64,
    values: Map<String, Value>
}

fn update_user_state (update_req: &UpdateRequest, user_values: Arc<Mutex<Vec<ValueAndTimestamp>>>) {
    //Lock the mutex, because we need to update the state.
    //The mutex is automatically unlocked when this function is done.
    let current_state: &mut Vec<ValueAndTimestamp> = &mut user_values.lock().unwrap();
    //Iterate all fields.
    for k in update_req.values.keys() {
        let name = k.clone();
        //Get the corresponding value for the field. Serde leaves the quoted around the field, since it does not consider it a String, so we trim them.
        let value = update_req.values.get(k).unwrap().to_string().trim_matches('\"').to_string();
        //Find the index of the value of the field in the user's state.
        let index = current_state.iter()
            .position(|u| u.name == name);
        if index != None {
            //We need to recheck the timestamp, since in some situation another thread might have updated the value after the previous check.
            if current_state[index.unwrap()].timestamp < update_req.timestamp {
                current_state[index.unwrap()] = ValueAndTimestamp {
                    name,
                    value,
                    timestamp: update_req.timestamp.clone()
                };    
            }
        } else {
            //If value is not found, add the new value to the state
            current_state.push(ValueAndTimestamp{
                name,
                value,
                timestamp: update_req.timestamp.clone()
            });
        }
    }
}

fn check_if_req_is_old (req: &UpdateRequest, current_state: Vec<ValueAndTimestamp>) -> bool {
    let mut return_check = true;
    //Iterate over all the fields
    for k in req.values.keys() {
        let name = k.clone();
        //Find the index of the value of the field
        let index = current_state.iter()
            .position(|u| u.name == name);
        //If it was found, check if the value is newer.
        if index != None {
            let old_value = &current_state[index.unwrap()];
            //If older value if found, we know that we need to update something.
            if old_value.timestamp < req.timestamp {
                return_check = false;
            }
        } else {
            //No value for the the given state was found, so we need to update
            return_check = false;
        }
    }
    return return_check;
}

//Read the file and asign the lines for threads.
fn read_lines_for_threads (filename: String, thread_count:usize) ->  Vec<Vec<String>> {
    //Initialize filereader and users. We assume that these users are all the users that the input will have.
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);
    //Lines for threads has the lines that each thread has to process. So at index 0 is thread 0's lines etc. etc.
    let mut lines_for_threads: Vec<Vec<String>> = Vec::new();
    let mut index: usize = 0;
    for _i in 0..thread_count {
        lines_for_threads.push(Vec::new());
    }
    //Add the lines to the matrix. The goal is to give every thread roughly the same amount of work, so we get good load balancing.
    //That way all threads finish at roughlt the same time. 
    for (_index, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        lines_for_threads[index].push(line);
        index += 1;
        if index == lines_for_threads.len() {
            index = 0;
        }
    }
    return lines_for_threads;
}

//Updates the state.
fn update (update_req: &UpdateRequest, user_values: Arc<Mutex<Vec<ValueAndTimestamp>>>) {
    /*
        We fetch the current state of the user.
        Mutex ensures that this thread is blocked if some other thread is using this users data.
        This way we do not copy old data and ensure thread safety.
        Since we only clone the data, the lock is immediately freed after the copy, so we do not block other threads while we only check for old request.
    */
    let current_state = user_values.lock().unwrap().clone();
    // For optimization, check based on the current state, if the update's timestamp is old.
    // If the update request only had old values, we know that we do not have to handle the request.
    if check_if_req_is_old(update_req, current_state) {
        return;
    }
    update_user_state(update_req, user_values);
}

fn parse_state_to_json (value_states: HashMap<String, Arc<Mutex<Vec<ValueAndTimestamp>>>>) -> String {
    let mut json_users:Map<String, Value> = Map::new();
    for k in value_states.keys() {
        let mut temp_map: Map<String, Value> = Map::new();
        let clone = value_states.get(k).unwrap().lock().unwrap().clone();
        if clone.len() == 0 {
            continue;
        }
        for v in clone {
            temp_map.insert(v.name.to_string(), Value::String(v.value));
        }
        json_users.insert(k.clone(), Value::Object(temp_map));
    }
    return serde_json::to_string_pretty(&json_users).unwrap();
}

//Main just parses the envs and passes it to updater
pub fn main () {
    //Configure envs
    let filename = std::env::args().nth(2).expect("no filename given");
    let thread_count_env = std::env::args().nth(3);
    let info_env = std::env::args().nth(4);
    let write_env = std::env::args().nth(5);
    let mut thread_count_arg: String = "4".to_string();
    let mut info_arg: String = "0".to_string();
    let mut write_arg: String = "0".to_string();
    if thread_count_env != None {
        thread_count_arg = thread_count_env.unwrap();
    }
    if info_env != None {
        info_arg = info_env.unwrap();
    }
    if write_env != None {
        write_arg = write_env.unwrap();
    }
    updater(filename, thread_count_arg, info_arg, write_arg)
}

pub fn updater (filename: String, thread_count_env: String, info_env: String, write_env: String) {
    //Configure envs
    let info: i32;
    let write: i32;
    let thread_count:usize;
    thread_count = thread_count_env.parse().expect("Incorrect thread count");
    info = info_env.parse().expect("Incorrect env");
    write = write_env.parse().expect("Incorrect env");
    //Init users
    let users = ["ab","bc","cd","de","ef","fg","gh","hi","ij","jk",
        "ba","cb","dc","ed","fe","gf","hg","ih","ji","kj"];
    //This is the data structure that maintains all the users' states.
    let mut value_states: HashMap<String, Arc<Mutex<Vec<ValueAndTimestamp>>>> = HashMap::new();
    //Set every user's state as empty
    for user in users {
        value_states.insert(user.to_string(), Arc::new(Mutex::new(Vec::new())));
    }
    let lines_for_threads: Vec<Vec<String>> = read_lines_for_threads(filename, thread_count);
    let now = Instant::now();
    //Spawn threads. Scope ensures that the main thread will not continue untill all the threads are done.
    thread::scope(|s| {
        for i in 0..thread_count.clone() {
            //Fetch the refrence to the thread's lines. We do not want to move the actual data since it is very large.
            let threads_lines = &lines_for_threads[i];
            s.spawn(|| {
                for line in threads_lines.iter() {
                    //Parse the line and take the refrence.
                    let update_req: &UpdateRequest = &serde_json::from_str(line.as_str()).unwrap();
                    //Get the Arc from the hashmap based on the update request's user.
                    let user_values = value_states.get(&update_req.user).unwrap();
                    //Call update with the refrence to update_req and Arc to the correct data. 
                    //Arc::clone clones the refrence, not the data itself. Arc ensures that the refrence is thread safe.
                    //No other thread has access to the given line, so it is safe as well.
                    update(update_req, Arc::clone(&user_values));
                }
            });
        }
    });
    //Parse the data to json format and ignore all users with empty state.
    let json = parse_state_to_json(value_states);
    println!("{}", json);
    let elapsed = now.elapsed();
    //Print the info about time and thread count if the env was set.
    if info == 1 {
        println!("Elapsed: {:.2?}", elapsed);
        println!("Thread count: {:?}", thread_count)
    }
    //Write the users' states to a file, if the env was set.
    if write == 1 {
        let path = "updater_output.txt";
        let mut output = File::create(path).unwrap();
        write!(output, "{}", json).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //Checks if output1.txt and updater_output.txt are EXACTLY the same. This way we can ensure that the updater worked correctly on the input1.txt file with correct format.
    fn check_files () -> bool {
        let correct_output = File::open("output1.txt").unwrap();
        let correct_output_reader = BufReader::new(correct_output);
        let updater_output = File::open("updater_output.txt").unwrap();
        let updater_output_reader = BufReader::new(updater_output);
        let mut correct_output_vec:Vec<String> = Vec::new();
        let mut updater_output_vec:Vec<String> = Vec::new();
        for (_index, line) in correct_output_reader.lines().enumerate() {
            let line = line.unwrap();
            correct_output_vec.push(line);
        }
        for (_index, line) in updater_output_reader.lines().enumerate() {
            let line = line.unwrap();
            updater_output_vec.push(line);
        }
        if correct_output_vec.len() != updater_output_vec.len() {
            return  false;
        }
        for i in 0..correct_output_vec.len() {
            if correct_output_vec[i] != updater_output_vec[i] {
                return false;
            }
        }
        return  true;
    }

    #[test]
    fn updater_1_thread () {
        updater("input1.txt".to_string(), "1".to_string(), "0".to_string(), "1".to_string());
        assert!(check_files())
    }
    #[test]
    fn updater_2_threads () {
        updater("input1.txt".to_string(), "2".to_string(), "0".to_string(), "1".to_string());
        assert!(check_files())
    }
    #[test]
    fn updater_3_threads () {
        updater("input1.txt".to_string(), "3".to_string(), "0".to_string(), "1".to_string());
        assert!(check_files())
    }
    #[test]
    fn updater_4_threads () {
        updater("input1.txt".to_string(), "4".to_string(), "0".to_string(), "1".to_string());
        assert!(check_files())
    }
}