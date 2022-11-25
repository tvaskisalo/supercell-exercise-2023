use std::fs::File;
use std::io::{BufRead, BufReader};
use serde::{Serialize, Deserialize};
use serde_json::{Value, Map};


#[derive(Serialize, Deserialize, Debug)]
struct UserState {
    name: String,
    values: Vec<ValueAndTimestamp>
}

#[derive(Serialize, Deserialize, Debug)]
struct ValueAndTimestamp {
    name: String,
    value: String,
    timestamp: i64
}

//Structures to handle JSON better
#[derive(Serialize, Deserialize, Debug, Clone)]
struct MakeFriendsRequest {
    user1: String,
    user2: String
}

#[derive(Serialize, Deserialize, Debug)]
struct DelFriendsRequest {
    user1: String,
    user2: String
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateRequest {
    user: String,
    timestamp: i64,
    values: Map<String, Value>
}

#[derive(Serialize, Deserialize, Debug)]
struct Broadcast {
    broadcast: Vec<String>,
    user: String,
    timestamp: i64,
    values: Map<String, Value>
}

//Makes two users friends.
fn make_friends (make_friends_req: MakeFriendsRequest, friends: &mut Vec<(String, String)>) {
    let user1 = make_friends_req.user1.clone();
    let user2 = make_friends_req.user2.clone();
    //If the users are already friends, do nothing.
    if friends.contains(&(user1.clone(), user2.clone())) || friends.contains(&(user2.clone(), user1.clone())) {
        return;
    }
    friends.push((user1.clone(), user2.clone()));
    return;
}

//Broadcasts update. Will return without printing if timestamp is outdated or user has no friends
fn update (update_req: UpdateRequest, friends: Vec<(String, String)>, user_values: &mut Vec<ValueAndTimestamp>) -> String {
    let mut users_friends: Vec<String> = Vec::new();
    let mut send = false;
    let mut updated_values: Map<String, Value> = Map::new();
    //Iterate over update request's fields
    for k in update_req.values.keys() {
        let name = k.clone();
        //Value of the field
        let value = update_req.values.get(k).unwrap();
        //Check if user's values has the field.
        let index = user_values.iter()
            .position(|u| u.name == name);
        if index != None {
            //Field was found, check timestamp.
            let old_value = &user_values[index.unwrap()];
            if old_value.timestamp >= update_req.timestamp {
                continue;
            }
            //Update user's values.
            user_values[index.unwrap()] = ValueAndTimestamp{
                name: name.clone(),
                value: value.to_string().clone(),
                timestamp: update_req.timestamp.clone()
            };
            send = true;
            //Add value to the broadcast
            updated_values.insert(name, value.clone());
        } else {
            //No field was found, add it to the state.
            user_values.push(ValueAndTimestamp{
                name: name.clone(),
                value: value.to_string().clone(),
                timestamp: update_req.timestamp.clone()
            });
            send = true;
            //Add value to the broadcast
            updated_values.insert(name, value.clone());
        }
    }
    //If timestamp is incorrect, return without printing
    if !send {
        return "".to_string();
    }
    //Add all user's friends to the list
    for (_index, friend) in friends.iter().enumerate() {
        if friend.0 == update_req.user {
            users_friends.push(friend.1.clone())
        }
        if friend.1 == update_req.user {
            users_friends.push(friend.0.clone())
        }
    }
    //If user has no friends, return without printing
    if users_friends.len() == 0 {
        return "".to_string();
    }
    let broadcast: Broadcast = Broadcast { broadcast: users_friends, user: update_req.user, timestamp: update_req.timestamp, values: updated_values };
    return serde_json::to_string(&broadcast).unwrap();
}
fn del_friends (del_friends_req: DelFriendsRequest, friends: &mut Vec<(String, String)>) {
    //Keep all friends except if the friends are given user1 and user2
    friends.retain(
        |f| 
        (f.0 != del_friends_req.user1 || f.1 != del_friends_req.user2)
        && 
        (f.0 != del_friends_req.user2 || f.1 != del_friends_req.user1)
    );
}
fn main () {
    //Get filename from env
    let filename = std::env::args().nth(2).expect("no filename given");
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);
    let value_states: &mut Vec<UserState> = &mut Vec::new();
    let friends: &mut Vec<(String, String)> = &mut Vec::new();
    //Iterate over all file's lines
    for (_index, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(',').collect();
        /* 
            This could be done with match, but with only 3 functions and oddly formatted string, this is easier.
            This way I don't have to parse the JSON to string now and make a complex data structure.
            We need to split the line by commas since, in theory, field values could have one of the command names as a value.
        */
        if parts[0].contains("make_friends") {
            //Parse JSON to MakeFriendsRequest
            let make_friends_req: MakeFriendsRequest = serde_json::from_str(&line).unwrap();
            make_friends(make_friends_req, friends)
        } else if line.contains("update") {
            //Parse JSON to UpdateRequest
            let update_req: UpdateRequest = serde_json::from_str(&line).unwrap();
            let name: &mut String = &mut update_req.user.clone();
            //Get the index for the user's values
            let mut user_value_index = value_states
                .iter()
                .position(|u| u.name == *name);
            //If given user does not have any values, add the user to the list.
            if user_value_index == None {
                value_states.push(UserState { 
                    name: name.clone().to_string(),
                    values: Vec::new() 
                });
                //Now user exists, so update the index.
                user_value_index = value_states
                    .iter()
                    .position(|u| u.name == *name);
            }
            //Fetch user's values
            let user_values = &mut value_states[user_value_index.unwrap()];
            let json = update(update_req, friends.clone(), &mut user_values.values);
            if json.len() > 0 {
                println!("{}", json);
            }
        } else if line.contains("del_friends") {
            //Parse JSON to DelFriendsRequest
            let del_friends_req: DelFriendsRequest = serde_json::from_str(&line).unwrap();
            del_friends(del_friends_req, friends)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cant_make_duplicate_friends () {
        let friend_req = MakeFriendsRequest {
            user1: "a".to_string(),
            user2: "b".to_string()
        };
        let friends: &mut Vec<(String, String)> = &mut Vec::new();
        make_friends(friend_req.clone(), friends);
        make_friends(friend_req.clone(), friends);
        assert!(friends.len()==1)
    }

    #[test]
    fn removing_friends_from_empty_list_is_okay () {
        let friends: &mut Vec<(String, String)> = &mut Vec::new();
        let del_req = DelFriendsRequest {
            user1: "a".to_string(),
            user2: "b".to_string()
        };
        del_friends(del_req, friends);
        assert!(friends.len() == 0)
    }

    #[test]
    fn update_returns_empty_string_with_no_friends () {
        let friends: Vec<(String, String)> = Vec::new();
        let mut values: Map<String, Value> = Map::new();
        values.insert("foo".to_string(), Value::String("bar".to_string()));
        let user_values: &mut Vec<ValueAndTimestamp> = &mut Vec::new();
        let update_req = UpdateRequest {
            user: "a".to_string(),
            timestamp: 100,
            values
        };
        let return_value = update(update_req, friends, user_values);
        assert_eq!(return_value, "".to_string());
    }

    #[test]
    fn update_updates_value_with_no_friends () {
        let friends: Vec<(String, String)> = Vec::new();
        let mut values: Map<String, Value> = Map::new();
        values.insert("foo".to_string(), Value::String("bar".to_string()));
        let user_values: &mut Vec<ValueAndTimestamp> = &mut Vec::new();
        let update_req = UpdateRequest {
            user: "a".to_string(),
            timestamp: 100,
            values
        };
        update(update_req, friends, user_values);
        assert!(user_values.len()==1);
        assert!(user_values[0].timestamp == 100)
    }
}