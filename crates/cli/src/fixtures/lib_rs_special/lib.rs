use std::collections::HashMap;
use std::fmt::Display;
use log::info;

/// A user entity with authentication capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub role: Role,
}

/// User roles in the system
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,
    User,
    Guest,
}

/// Current status of a user
#[derive(Debug)]
pub enum Status {
    Active,
    Inactive,
    Pending,
    Suspended,
}

/// Trait for items that can be identified
pub trait Identifiable {
    fn id(&self) -> u32;
}

/// Trait for items that can be validated
pub trait Validatable {
    fn is_valid(&self) -> bool;
}

impl Identifiable for User {
    fn id(&self) -> u32 {
        self.id
    }
}

impl Validatable for User {
    fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.email.contains('@')
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "Administrator"),
            Role::User => write!(f, "User"),
            Role::Guest => write!(f, "Guest"),
        }
    }
}

/// Creates a new user with the given details
pub fn create_user(name: String, email: String, role: Role) -> User {
    User {
        id: generate_id(),
        name,
        email,
        role,
    }
}

/// Validates an email address format
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

/// Gets the count of users in a collection
pub fn get_user_count(users: &HashMap<u32, User>) -> usize {
    users.len()
}

/// Finds a user by their ID
pub fn find_user_by_id(users: &HashMap<u32, User>, id: u32) -> Option<&User> {
    users.get(&id)
}

/// Generates a unique ID (simplified version)
fn generate_id() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

/// Activates a user account
pub fn activate_user(user: &mut User) {
    // Implementation would set user status to active
    info!("User {} activated", user.name);
}
