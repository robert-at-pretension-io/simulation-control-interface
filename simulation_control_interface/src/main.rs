#![feature(vec_remove_item)]
#![feature(half_open_range_patterns)]
#![feature(exclusive_range_pattern)]
#![feature(drain_filter)]
#![allow(dead_code)]

use storage_backend;


use actix_web::{web, App, HttpServer, HttpResponse};
use log::info;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use vectors::Distance;

use dotenv;
use plotters::palette::Srgb;
use plotters::prelude::*;

// use rand_pcg::Mcg128Xsl64;
// use rand_core::{SeedableRng,RngCore};

use serde::{Deserialize};


use std::env;

/// # What I want to accomplish:
/// -> a feature rich and extensible simulation engine that can be started, paused, given initial state, reload old states
/// -> clear separation of boundaries: meaning that data can be accessible from other programs. This will be accomplished by using a database to store all data generated by rounds of the simulation. This is especially useful for running statistical measurements/algorithms on the data. Furthermore, 'normal' text files will be used for the configuration (actually, maybe this should also be in the database... hmmm)
/// -> the program will have "modes" of running that are determined by configuration files/command line input. For instance, one of the modes of operation will be a step-wise runtime that will only run the next round when given a command. Another mode will be continuing till a predicate is met.



async fn create_user() -> HttpResponse{
let conn = storage_backend::establish_connection();
let result = storage_backend::create_user(&conn).await;

match result {
    Ok(user) => HttpResponse::Ok().json(user),
    Err(err) => HttpResponse::NotAcceptable().message_body(err.to_string().into())
}



}



#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    // Load config file/read from the command line
    // potential preferances will include:
    // * max number of users in simulation
    // * main loop time 


    // Setup the asynchronous rest/routing capabilities so that the simulation can be controlled through the internet *gasp*. The reason the actix-web is setup here is so that shared state (db connection?) can be shared amongst all of the routes

    HttpServer::new(|| {
        App::new()
            .service(
                web::resource("/create_user")
                .route(web::get().to( create_user )
            )
        ) })
    .bind("127.0.0.1:8080")?
    .run()
    .await

    // right here is where the main program will loop every minute or couple of minutes to match people who have exited their conversation and are ready for a new conversation
    // loop {
    //     //
    // }

    // let mut my_scenario = Scenario::new(50, Some(500));
    // my_scenario.start();
}



fn load_config(list_of_env_var_keys: Vec<String>) -> HashMap<String, String> {
    // first we will load the .env variable
    dotenv::dotenv().ok();

    let mut return_hash: HashMap<String, String> = HashMap::new();

    env::vars().for_each(|(k, v)| {
        if list_of_env_var_keys.contains(&k) {
            return_hash.insert(k.into(), v.into());
        }
    });

    return_hash
}

trait Model {}

trait Simulation {
    fn start() {
        Self::initial_setup();
        while Self::stopping_condition() {
            Self::before_round();
            Self::next_round();
            Self::after_round();
        }
    }

    fn initial_setup();

    fn stopping_condition() -> bool;

    fn before_round();

    fn next_round();

    fn after_round();
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
enum Quadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// This will refer to the containing boundary that a person will belong to.
struct QuadrantUtils {
    top_left_count: u32,
    top_right_count: u32,
    bottom_left_count: u32,
    bottom_right_count: u32,

    inner_quadrant_boundaries: HashMap<Quadrant, BoundingBox>,
    //boundary : BoundingBox
}

impl QuadrantUtils {
    fn new(boundary: BoundingBox) -> Self {
        let mut inner_quadrant_boundaries = HashMap::<Quadrant, BoundingBox>::new();
        let mid_vertical_boundary = boundary.right / 2.0;
        let mid_horizontal_boundary = boundary.top / 2.0;

        let top_left = BoundingBox {
            top: boundary.top,
            bottom: mid_horizontal_boundary,
            left: boundary.left,
            right: mid_vertical_boundary,
        };

        let top_right = BoundingBox {
            top: boundary.top,
            bottom: mid_horizontal_boundary,
            left: mid_vertical_boundary,
            right: boundary.right,
        };

        let bottom_left = BoundingBox {
            top: mid_horizontal_boundary,
            bottom: boundary.bottom,
            left: boundary.left,
            right: mid_vertical_boundary,
        };

        let bottom_right = BoundingBox {
            top: mid_horizontal_boundary,
            bottom: boundary.bottom,
            left: mid_vertical_boundary,
            right: boundary.right,
        };

        inner_quadrant_boundaries.insert(Quadrant::TopLeft, top_left);
        inner_quadrant_boundaries.insert(Quadrant::TopRight, top_right);
        inner_quadrant_boundaries.insert(Quadrant::BottomLeft, bottom_left);
        inner_quadrant_boundaries.insert(Quadrant::BottomRight, bottom_right);

        QuadrantUtils {
            top_left_count: 0,
            top_right_count: 0,
            bottom_left_count: 0,
            bottom_right_count: 0,
            inner_quadrant_boundaries,
        }
    }

    fn count_contained_points(&mut self, points: Vec<Point>) {
        let mut count = 0;

        for p1 in &points {
            if self
                .inner_quadrant_boundaries
                .get(&Quadrant::TopLeft)
                .unwrap()
                .contains_point(Point::from(p1))
            {
                self.top_left_count += 1;
                count += 1;
                continue;
            }
            if self
                .inner_quadrant_boundaries
                .get(&Quadrant::TopRight)
                .unwrap()
                .contains_point(Point::from(p1))
            {
                self.top_right_count += 1;
                count += 1;
                continue;
            }
            if self
                .inner_quadrant_boundaries
                .get(&Quadrant::BottomLeft)
                .unwrap()
                .contains_point(Point::from(p1))
            {
                self.bottom_left_count += 1;
                count += 1;
                continue;
            }
            if self
                .inner_quadrant_boundaries
                .get(&Quadrant::BottomRight)
                .unwrap()
                .contains_point(Point::from(p1))
            {
                self.bottom_right_count += 1;
                count += 1;
                continue;
            }
        }
        assert!(count == points.len());
    }

    fn return_random_point_in_smallest_quadrant(self) -> Option<Point> {
        if self.top_left_count == 0
            && self.top_right_count == 0
            && self.bottom_left_count == 0
            && self.bottom_right_count == 0
        {
            return None;
        } else {
            info!(
                "tl: {}, tr: {}, bl: {}, br: {}",
                self.top_left_count,
                self.top_right_count,
                self.bottom_left_count,
                self.bottom_right_count
            );
        }

        let v = vec![
            (self.bottom_left_count, Quadrant::BottomLeft),
            (self.bottom_right_count, Quadrant::BottomRight),
            (self.top_right_count, Quadrant::TopRight),
            (self.top_left_count, Quadrant::TopLeft),
        ];

        let quadrant = return_index_of_min_value(v);
        info!("choosing quadrant: {:?}", quadrant);

        let rando_point = Point::new_bounded_point(
            &self
                .inner_quadrant_boundaries
                .get(&quadrant)
                .unwrap()
                .clone(),
        );

        info!("{}", rando_point);

        Some(rando_point)
    }
}

fn return_index_of_min_value(mut v: Vec<(u32, Quadrant)>) -> Quadrant {
    if v[0].0 == v[1].0 && v[1].0 == v[2].0 && v[2] == v[3] {
        info!("yatzee")
    };

    v.sort_by(|a, b| {
        let a = a.0;
        let b = b.0;
        a.partial_cmp(&b).unwrap()
    });

    info!("{:?}", v);

    v[0].1
}

/// The metastructure that contains the implementation of a generic Scenario, including:
/// 1. how many actors are in the system
/// 2. how the system will behave each round
/// 3. the stopping conditions of the system
struct Scenario {
    tribe: Vec<Person>,
    size: usize,
    total_rounds: u16,
    current_round: u16,
    shared_location_history: Arc<Mutex<History>>,
    interaction_history: HashMap<Uuid, Vec<Arc<Mutex<Rating>>>>,

    /// This is used for determining how long everyone's memory of interactions is. The main usage is for not allowing for continual matching with the same person that they've JUST interacted with
    recent_interactions: usize,
    boundary: BoundingBox,
}

impl Scenario {
    /// The default number of total possible rounds of interactions is 1000
    ///
    //// depending on the role of the dice, an individual will only interact a percentage of the time
    ///
    /// it will be interesting to study what happens to the individuals that interact with the system with high probability vs low probability
    fn new(size: usize, total_rounds: Option<u16>) -> Self {
        let recent_interactions: usize = 10;

        let boundary = BoundingBox {
            left: 0.0,
            right: 1.0,
            top: 1.0,
            bottom: 0.0,
        };

        // Default value of total rounds is 1000
        let total_rounds = match total_rounds {
            Some(total_rounds) => total_rounds,
            None => 1000,
        };

        let shared_location_history = Arc::new(Mutex::new(History::new()));
        let mut interaction_history = HashMap::<Uuid, Vec<Arc<Mutex<Rating>>>>::new();

        info!("Starting scenario with {} people.", size);
        let mut tribe: Vec<Person> = Vec::with_capacity(size);
        for _ in 0..size {
            let p = Person::new(shared_location_history.clone());
            interaction_history.insert(p.uuid.clone(), Vec::<Arc<Mutex<Rating>>>::new());
            tribe.push(p)
        }
        info!("The scenario initialzed successfully!");
        let mut return_scenario = Scenario {
            tribe,
            size,
            total_rounds,
            current_round: 0,
            shared_location_history,
            interaction_history,
            recent_interactions,
            boundary,
        };
        return_scenario.add_preference();
        return_scenario
    }

    fn percentage_of_last_n_interactions_bad(&self, n: usize, person: Uuid) -> Option<f64> {
        let history = self.interaction_history.get(&person).unwrap();
        if history.len() >= n {
            return None;
        };

        let bad_interactions_out_of_last_n = history.iter().rev().take(n).fold(0.0f64, |acc, r| {
            let rating = r.lock().unwrap().you_rate_other_person(person);
            if rating < 0 {
                acc + 1.0
            } else {
                acc
            }
        });

        let n = n as f64;

        Some(bad_interactions_out_of_last_n / n)
    }

    fn return_locations_of_people_with_negative_interactions(&self, myself: &Uuid) -> Vec<Point> {
        let mut bad_interaction_points = Vec::<Point>::new();

        for r in self.interaction_history.get(&myself).unwrap() {
            let rating = r.lock().unwrap();
            if rating.other_person_rates_you(myself) < 0 {
                let other_person = &rating.other_person(&myself);
                let their_location = self.return_last_location_by_uuid(other_person);
                bad_interaction_points.push(their_location)
            }
        }
        bad_interaction_points
    }

    fn return_last_n_interactions_with_uuid(&self, person: &Uuid, n: usize) -> Vec<Uuid> {
        let mut last_n = Vec::<Uuid>::new();

        let vec = self.interaction_history.get(&person).unwrap();

        if vec.len() > n {
            //get the last n uuids
            for i in (vec.len() - n)..vec.len() {
                last_n.push(vec.get(i).unwrap().lock().unwrap().other_person(person))
            }
        } else {
            //or return whatever interactions there are
            for val in vec {
                last_n.push(val.lock().unwrap().other_person(person))
            }
        }

        last_n
    }

    fn pair_by_distance_excluding_recent_interactions(
        &self,
        person: &Uuid,
        choices: &Vec<Uuid>,
    ) -> Option<Vec<Uuid>> {
        assert!(choices.len() > 0);

        let mut choices = choices.clone();
        choices.remove_item(&person);

        //first remove all recent interactions from the potential matches

        let recent_interaction_uuid =
            self.return_last_n_interactions_with_uuid(person, self.recent_interactions);
        let my_location = self.return_last_location_by_uuid(&person);

        //first filter out the uuids of recent interactions!

        choices.drain_filter(|item| recent_interaction_uuid.contains(item));

        if choices.len() == 0 {
            return None;
        }

        let mut best_uuid_so_far = choices.first().unwrap();
        let best_point = self.return_last_location_by_uuid(&best_uuid_so_far);
        let mut smallest_distance_so_far = my_location.squared_distance(&best_point);

        for uuid in choices.iter().skip(1) {
            let cur_point = self.return_last_location_by_uuid(uuid);
            let cur_distance = my_location.squared_distance(&cur_point);
            if cur_distance < smallest_distance_so_far {
                smallest_distance_so_far = cur_distance;
                best_uuid_so_far = uuid;
            }
        }
        Some(vec![person.to_owned(), best_uuid_so_far.to_owned()])
    }

    fn return_last_location_by_uuid(&self, person: &Uuid) -> Point {
        self.shared_location_history
            .lock()
            .unwrap()
            .get_latest_point_by_uuid(person.to_owned())
    }

    fn make_graph(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut my_points = Vec::<(f64, f64, Srgb)>::new();

        for p in &self.tribe {
            let (x, y) = p.last_location().as_tuple();
            let color = p.color;
            my_points.push((x, y, color));
        }

        assert!(my_points.len() > 0);

        let root = BitMapBackend::new(&filename, (640, 480)).into_drawing_area();

        root.fill(&RGBColor(255, 255, 255))?;

        let root = root.apply_coord_spec(RangedCoord::<RangedCoordf64, RangedCoordf64>::new(
            0f64..1f64,
            0f64..1f64,
            (0..640, 0..480),
        ));

        let color_dot = |x: f64, y: f64, color: Srgb| {
            return EmptyElement::at((x, y))
                + Circle::new((0, 0), 3, ShapeStyle::from(&color).filled());
            // + Text::new(
            //     format!("({:.2},{:.2})", x, y),
            //     (10, 0),
            //     ("sans-serif", 15.0).into_font(),
            // );
        };

        for p in my_points {
            root.draw(&color_dot(p.0, p.1, p.2))?;
        }

        info!("Should have created the file at: {}", filename);
        Ok(())
    }

    fn make_line_graph(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut my_points = Vec::<(Point, Point, Srgb)>::new();

        for p in &self.tribe {
            //get the location history

            let p1: Point;
            let p2: Point;

            match p.last_two_locations() {
                Some(points) => {
                    p1 = points.0;
                    p2 = points.1;
                }
                None => continue,
            }
            let color = p.color;
            my_points.push((p1, p2, color));
        }

        if my_points.is_empty() {
            return Ok(());
        }

        let root = BitMapBackend::new(&filename, (640, 480)).into_drawing_area();

        root.fill(&RGBColor(255, 255, 255))?;

        let root = root.apply_coord_spec(RangedCoord::<RangedCoordf64, RangedCoordf64>::new(
            0f64..1f64,
            0f64..1f64,
            (0..640, 0..480),
        ));

        let color_dot = |p1: Point, p2: Point, color: Srgb| {
            return PathElement::new(
                vec![(p1.x, p1.y), (p2.x, p2.y)],
                ShapeStyle::from(&color).filled(),
            );

            // + Text::new(
            //     format!("({:.2},{:.2})", x, y),
            //     (10, 0),
            //     ("sans-serif", 15.0).into_font(),
            // );
        };

        let mark_endpoint = |p2: Point, color: Srgb| {
            return Cross::new((p2.x, p2.y), 4, ShapeStyle::from(&color).filled());
        };

        for p in my_points {
            root.draw(&color_dot(p.0, p.1, p.2))?;
            root.draw(&mark_endpoint(p.1, p.2))?;
        }

        info!("Should have created the file at: {}", filename);
        Ok(())
    }

    fn person_by_uuid<'a>(&'a mut self, uuid: Uuid) -> &'a mut Person {
        let index = self.tribe.iter().position(|p| p.uuid == uuid).unwrap();
        &mut self.tribe[index]
    }

    fn add_interaction(&mut self, person_a: Uuid, person_b: Uuid) {
        //info!("person a ({}) and person b ({}) are interacting", person_a, person_b);

        let a_rates_b = self.person_by_uuid(person_a).lookup_rating(&person_b);
        let b_rates_a = self.person_by_uuid(person_b).lookup_rating(&person_a);

        //info!("person a rates person b: {}", a_rates_b);
        //info!("person b rates person a: {}", b_rates_a);

        let rating = Rating::new(person_a, person_b, a_rates_b, b_rates_a);
        let shared_rating = Arc::new(Mutex::new(rating));

        match self.interaction_history.get_mut(&person_a) {
            None => {
                self.interaction_history
                    .insert(person_a, vec![shared_rating.clone()]);
            }
            Some(v) => v.push(shared_rating.clone()),
        }

        match self.interaction_history.get_mut(&person_a) {
            None => {
                self.interaction_history
                    .insert(person_a, vec![shared_rating.clone()]);
            }
            Some(v) => v.push(shared_rating.clone()),
        }
    }

    fn stage_update_locations(&mut self) {
        self.tribe.iter().for_each(|p| {
            p.stage_location_update(
                self.shared_location_history.clone(),
                self.interaction_history.get(&p.uuid).unwrap().clone(),
            )
        });
    }

    fn push_updated_locations(&mut self) {
        let mut uuid_to_point = HashMap::<Uuid, Option<Point>>::new();
        self.tribe.iter().for_each(|p| {
            uuid_to_point.insert(
                p.uuid,
                p.shared_location_history
                    .lock()
                    .unwrap()
                    .return_staged_point(p.uuid),
            );
        });

        self.tribe
            .iter()
            .for_each(|p| match uuid_to_point.get(&p.uuid).unwrap() {
                Some(point) => {
                    p.shared_location_history
                        .lock()
                        .unwrap()
                        .add_point_to_uuid(Point::from(point), p.uuid);
                }
                None => (),
            })
    }

    /// This is where the magic happens.
    /// This function decides who will be matched each "round".
    /// Though, before this project goes into production, the concept of a round must be altered so it can be invoked asynchronously. Essentially, whenever someone attempts to match with a new person, the system **must** know who is currently online and create a round.
    fn next_round(&mut self) {
        info!(
            "\n----------------------------\nBeginning round # {}\n",
            self.current_round
        );

        let mut people_online = self.determine_who_is_online();
        //These steps need to be separate so that all updates are done at once, not effecting the results of previous updates occuring in the same round
        self.stage_update_locations();
        self.push_updated_locations();

        let num_people_online = people_online.len();
        while people_online.len() >= 2 {
            let pair: Vec<Uuid>;

            let first_person = people_online.first().unwrap();
            let last_rating = self.interaction_history.get(&first_person).unwrap();

            // .last() {
            //     Some(v) => v.lock().unwrap().get_combined_rating(),
            //     None => -1,
            // };

            // if the percentage of the last n interactions is below some threshold, send them to a random quadrant
            let n = 5;

            //let percent_bad = self.percentage_of_last_n_interactions_bad(n, first_person.clone() );

            if last_rating.len() > n {
                // if the previous interaction was bad, find the quadrant with the least number of bad interactions and set the last location of the person to some point in that quadrant

                let bad_points =
                    self.return_locations_of_people_with_negative_interactions(first_person);
                let mut quadrant = QuadrantUtils::new(self.boundary);
                quadrant.count_contained_points(bad_points);
                let new_position = quadrant.return_random_point_in_smallest_quadrant();

                match new_position {
                    Some(new_position) => {
                        self.shared_location_history
                            .lock()
                            .unwrap()
                            .add_point_to_uuid(new_position, *first_person);
                    }
                    None => {}
                }

                //pair = choose_random_partner_for(first_person.clone(), &people_online);
            }
            match self.pair_by_distance_excluding_recent_interactions(first_person, &people_online)
            {
                None => pair = choose_random_partner_for(first_person.clone(), &people_online),
                Some(val) => pair = val,
            }

            assert!(pair.len() == 2);
            find_and_remove(&pair, &mut people_online);

            let person_a = pair[0];
            let person_b = pair[1];
            self.add_interaction(person_a, person_b);
        }

        info!(
            "At round {}, {} people were paired.",
            self.current_round.clone(),
            num_people_online
        );
        self.make_line_graph(format!("round_{}a.png", self.current_round))
            .unwrap();
        self.make_graph(format!("round_{}b.png", self.current_round))
            .unwrap();

        self.current_round += 1;
    }
    fn add_preference(&mut self) {
        let mut uuids = Vec::<Uuid>::new();
        assert!(self.tribe[0].preference.len() == 0);
        for person in &self.tribe {
            uuids.push(person.uuid.clone());
        }
        for person in &mut self.tribe {
            let mut rng = thread_rng();
            let split_index = uuids.clone().iter().position(|x| x == &person.uuid);

            let split_index = match split_index {
                Some(n) => n,
                None => panic!("shit... uh.."),
            };
            let mut personal_preference = uuids.clone();

            personal_preference.remove(split_index);

            rng.shuffle(&mut personal_preference[..]);
            assert!(personal_preference.len() < uuids.len());
            assert!(!personal_preference.contains(&person.uuid.clone()));
            person.add_preference(personal_preference.to_owned())
        }
    }
    /// roll `size` dice to determine the pairing of people for a particular "round"
    fn determine_who_is_online(&mut self) -> Vec<Uuid> {
        let mut rng = thread_rng();
        let online_people: &mut Vec<Uuid> = &mut Vec::<Uuid>::new();
        for i in 0..self.size {
            let r = rng.next_f64();
            if self.tribe[i].online_percentage > r {
                online_people.push(self.tribe[i].uuid.clone())
            }
        }
        online_people.to_owned()
    }

    /// TODO: add other stopping conditions
    fn start(&mut self) {
        while self.current_round < self.total_rounds {
            self.next_round();
        }
    }
}

/// This is the location history of each person for each round that they participate
struct History {
    by_uuid: HashMap<Uuid, Vec<Point>>,
    staged_point_by_uuid: HashMap<Uuid, Option<Point>>,
}
impl History {
    fn new() -> History {
        let uuid_hash = HashMap::<Uuid, Vec<Point>>::new();
        let staged_point_by_uuid = HashMap::<Uuid, Option<Point>>::new();

        History {
            by_uuid: uuid_hash,
            staged_point_by_uuid,
        }
    }

    fn stage_point(&mut self, uuid: Uuid, point: Point) {
        assert!(
            !self.staged_point_by_uuid.contains_key(&uuid)
                || self.staged_point_by_uuid.get(&uuid).unwrap().is_none()
        );

        self.staged_point_by_uuid.insert(uuid, Some(point));
    }

    fn return_staged_point(&mut self, uuid: Uuid) -> Option<Point> {
        let maybe_point: Option<Point>;

        if !self.staged_point_by_uuid.contains_key(&uuid) {
            return None;
        }

        match self.staged_point_by_uuid.get(&uuid).unwrap() {
            Some(point) => maybe_point = Some(Point::from(point)),
            None => maybe_point = None,
        }

        match maybe_point {
            Some(_) => {
                self.staged_point_by_uuid.insert(uuid, None);
            }
            None => (),
        }
        maybe_point
    }

    fn add_point_to_uuid(&mut self, point: Point, uuid: Uuid) {
        let contains_key = self.by_uuid.contains_key(&uuid);

        if !contains_key {
            info!(
                "This is the first time that the uuid '{}' shows up in the history.",
                uuid
            );
            self.by_uuid.insert(uuid, Vec::<Point>::new());
        }
        //info!("Inserting new point {} into uuid {}", &point, &uuid);
        self.by_uuid.get_mut(&uuid).unwrap().push(point);
    }

    fn get_latest_point_by_uuid(&self, uuid: Uuid) -> Point {
        self.by_uuid.get(&uuid).unwrap().last().unwrap().into()
    }

    fn get_latest_n_points_by_uuid(&self, n: usize, uuid: Uuid) -> Option<Vec<Point>> {
        let vec = self.by_uuid.get(&uuid).unwrap();

        if vec.len() < n {
            return None;
        }

        let points: Vec<Point> = vec.iter().rev().take(n).map(|p| Point::from(p)).collect();

        Some(points)
    }
}

struct Person {
    /// This is used for uniquely referring to people within the scenario context.
    uuid: Uuid,

    /// This will be the color that a person is colored... will be random

    //color : String,

    /// used to calculate where the person is in "friend-space". It is initialized with a random point (x,y) | x,y \in [0,1] \subseteq \mathbb{R}
    shared_location_history: Arc<Mutex<History>>,

    // /// After an interaction, the person will store good interactions in here.
    // /// Maps between the Uuid of a friend and the combined "preference" of each person for the other
    // interaction_history : HashMap<Uuid, Vec<Rating>>,
    /// This is the theoretical ranking of all of the people in the system based on preference of interaction.
    /// For the purpose of this simulation, the distribution of the people will initially be completely random.
    /// After attempting the simulation with the most basic assumptions, the preference might be mutable in the future ...
    /// People change after all ...
    ///
    /// # Todo :
    /// * Make the preference update if the other person did NOT have a good interaction with them
    preference: Vec<Uuid>,

    /// contains all the possible ratings this person can give another person.
    rating_range: Range<i32>,

    /// Each person has this initialized at the beginning of the scenario.. It represents how selective they are to consider someone else their friend
    friendship_threshold: u32,

    /// the relative time that a user is online, where 1.0 indicates that the person is always online
    online_percentage: f64,

    /// this is the color the person will show up on the graph
    color: Srgb,
}

impl Person {
    fn new(shared_location_history: Arc<Mutex<History>>) -> Self {
        let p = Point::new();
        let start = -10;
        let end = 11;
        let rating_range: Range<i32> = Range { start, end };
        let friendship_threshold = rand::thread_rng().gen_range(0u32, end as u32);
        let online_percentage = 1.0; //rand::thread_rng().gen_range(0.5, 1.0);
        let uuid = Uuid::new_v4();

        let r = rand::thread_rng().gen_range(0.0, 1.0);
        let g = rand::thread_rng().gen_range(0.0, 1.0);
        let b = rand::thread_rng().gen_range(0.0, 1.0);

        let color = Srgb::new(r, g, b);

        assert!(rating_range.contains(&(friendship_threshold as i32)));
        info!("Creating person {}", uuid);
        info!("This person has a friendship threshold of: {}. They will permit any interactions with a combined rating *2 this number to affect their movement!", &friendship_threshold);

        // This is the starting point each person exists at. It is a randomly determined number from Point::new()
        shared_location_history
            .lock()
            .unwrap()
            .add_point_to_uuid(p, uuid);

        Person {
            friendship_threshold,
            online_percentage,
            uuid,
            shared_location_history,
            rating_range,
            preference: Vec::<Uuid>::new(),
            color,
        }
    }

    fn last_location(&self) -> Point {
        self.shared_location_history
            .lock()
            .unwrap()
            .get_latest_point_by_uuid(self.uuid)
    }

    fn last_two_locations(&self) -> Option<(Point, Point)> {
        match self
            .shared_location_history
            .lock()
            .unwrap()
            .get_latest_n_points_by_uuid(2, self.uuid)
        {
            Some(vec) => Some((vec[0], vec[1])),
            None => None,
        }
    }

    /// Each person has their own friendship threshold from 0 to the highest rating. This will effect their movement within the system asymmetrically
    /// Todo: Make it so that updates are put into a "staging" mode so that the scenario can update the new round's location all at once so that EACH individual is updating their location based on previous rounds and NOT on the updated location of others in the current round.
    fn stage_location_update(
        &self,
        shared_location_history: Arc<Mutex<History>>,
        shared_interation_history: Vec<Arc<Mutex<Rating>>>,
    ) {
        // calculate new point based on locations of all previous interactions
        // the problem with this is that for the current implementation, each person keeps track of their location

        let myself = self.uuid;

        if shared_interation_history.len() > 0 {
            let mut weighted_vec = Vec::<(f64, f64, Arc<Point>)>::new();

            for interation in shared_interation_history {
                let other_person = interation.lock().unwrap().other_person(&myself);
                let they_rate_me =
                    interation.lock().unwrap().other_person_rates_you(&myself) as f64;
                let i_rate_them = interation.lock().unwrap().you_rate_other_person(myself) as f64;

                let combined_rating = they_rate_me + i_rate_them;

                let new_location = Arc::new(
                    shared_location_history
                        .lock()
                        .unwrap()
                        .get_latest_point_by_uuid(other_person),
                );

                //info!("This interaction seemed to have been a good one. the combined rating was {} which is above my friendship threshold: {}", (they_rate_me+i_rate_them), self.friendship_threshold);

                //info!("The person's friendship threshold is {}.", self.friendship_threshold);

                if combined_rating > (2.0 * self.friendship_threshold as f64) {
                    //info!("This interaction seemed to have been a good one. the combined rating was {} which is above my friendship threshold: {}", (they_rate_me+i_rate_them), self.friendship_threshold);
                    weighted_vec.push((they_rate_me, i_rate_them, new_location.clone()));
                }
            }

            if weighted_vec.len() > 0 {
                let weighted_point = Point::weighted_average(weighted_vec);

                //self.staged_history = Some(weighted_point.into());
                self.shared_location_history
                    .lock()
                    .unwrap()
                    .stage_point(myself, weighted_point);
            }
        }
    }

    /// this cannot be initialized by the person themselves, the simulation environment must bestow this upon the person
    /// ... Did I just solve the nature vs nuture problem?

    fn add_preference(&mut self, preference: Vec<Uuid>) {
        self.preference = preference;
    }

    ///the preference will always be converted to a "rating" of the person between -10 (extremely bad interaction) to 10 (extremely good interaction)
    fn lookup_rating(&self, person: &Uuid) -> i32 {
        let index = self.preference.
        iter().
        position(|p| p == person).
        expect("without a doubt, the preference vector must contain the person otherwise an implementation error occurred. The preference vector is initialized at the beginning of the simulation, each person has a vector of hypothetical preferences for every other person.");

        //info!("\nThe raw rating is: {}", index);

        let proportion = 1.0 - (index as f64 / self.preference.len() as f64);

        //info!("The proportion of the way through the preference is: {}. As the total preference length is: {}", proportion, self.preference.len());

        let new_index = proportion * self.rating_range.len() as f64;

        //info!("And so the rating should be... {}", new_index);

        let new_index = ((new_index as i32) - (self.rating_range.len() as i32 / 2)) as i32 - 1;

        //info!("After shifting it is: {}\n", new_index);

        new_index.clone()
    }
}

#[derive(Copy, Clone)]
struct BoundingBox {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl BoundingBox {
    fn contains_point(&self, p: Point) -> bool {
        p.x <= self.right && p.x > self.left && p.y >= self.bottom && p.y < self.top
    }
}
#[derive(Copy, Clone, Deserialize)]

struct Point {
    x: f64,
    y: f64,
}

impl vectors::Distance for Point {
    type Scalar = f64;

    fn squared_distance(&self, rhs: &Self) -> Self::Scalar {
        (self.x - rhs.x).powi(2) + (self.y - rhs.y).powi(2)
    }
}

impl From<&Point> for Point {
    fn from(point: &Point) -> Self {
        Point {
            x: point.x.clone(),
            y: point.y.clone(),
        }
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

impl Point {
    fn new() -> Self {
        let mut rng = thread_rng();
        let x: f64 = rng.gen_range(0f64, 1f64);
        let y: f64 = rng.gen_range(0f64, 1f64);
        Point { x, y }
    }

    fn new_bounded_point(boundry: &BoundingBox) -> Self {
        let mut rng = thread_rng();
        let x: f64 = rng.gen_range(boundry.left, boundry.right);
        let y: f64 = rng.gen_range(boundry.bottom, boundry.top);
        Point { x, y }
    }

    fn as_tuple(self) -> (f64, f64) {
        (self.x, self.y)
    }

    fn weighted_average(weighted_vec: Vec<(f64, f64, Arc<Point>)>) -> Point {
        let mut total: f64 = 0.0;

        for v in &weighted_vec {
            total = total + v.0 + v.1;
        }

        let x = weighted_vec.iter().fold(0 as f64, |acc, elem| {
            acc + ((elem.0 + elem.1) * elem.2.x) / (total as f64)
        });
        let y = weighted_vec.iter().fold(0 as f64, |acc, elem| {
            acc + ((elem.0 + elem.1) * elem.2.y) / (total as f64)
        });

        //info!("The weighted point turns out to be ({},{}).",&x,&y);

        Point { x, y }
    }
}

fn find_and_remove<'a, T: PartialEq>(find_these: &Vec<T>, remove_from_here: &'a mut Vec<T>) {
    assert!(find_these.len() <= remove_from_here.len());
    let cur_length = remove_from_here.len();
    for val in find_these {
        assert!(remove_from_here.contains(&val));
        let index = remove_from_here
            .iter()
            .position(|elem| elem == val)
            .unwrap();
        remove_from_here.remove(index);
    }
    assert!(cur_length == find_these.len() + remove_from_here.len());
}

fn choose_random_partner_for(person: Uuid, possibilities: &Vec<Uuid>) -> Vec<Uuid> {
    assert!(possibilities.len() >= 2);

    let mut local = possibilities.clone();

    let mut my_choices = Vec::<Uuid>::new();
    my_choices.push(person);
    local.remove_item(&person);

    let mut rng = rand::thread_rng();

    while my_choices.len() < 2 {
        let index = rng.gen_range(0 as usize, local.len());
        my_choices.push(local.get(index).unwrap().clone());
        local.remove(index);
    }

    assert!(my_choices.len() + local.len() == possibilities.len());

    my_choices
}

/// function to choose n from vec of size m without replacement
/// returns a tuple of (chosen_ones, remaining list)

struct Rating {
    person_a: Uuid,
    person_b: Uuid,
    a_rates_b: i32,
    b_rates_a: i32,
}

impl Rating {
    fn new(person_a: Uuid, person_b: Uuid, a_rates_b: i32, b_rates_a: i32) -> Rating {
        Rating {
            person_a,
            person_b,
            a_rates_b,
            b_rates_a,
        }
    }
    fn other_person(&self, you: &Uuid) -> Uuid {
        if you == &self.person_a {
            return self.person_b.clone();
        } else {
            return self.person_a.clone();
        }
    }

    fn you_rate_other_person(&self, you: Uuid) -> i32 {
        if you == self.person_a {
            return self.a_rates_b;
        } else {
            return self.b_rates_a;
        }
    }

    fn get_combined_rating(&self) -> i32 {
        self.a_rates_b + self.b_rates_a
    }

    fn other_person_rates_you(&self, you: &Uuid) -> i32 {
        if you == &self.person_a {
            return self.b_rates_a;
        } else {
            return self.a_rates_b;
        }
    }
}
