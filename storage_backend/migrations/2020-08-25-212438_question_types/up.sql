CREATE TABLE numeric_types (
    id BIGSERIAL PRIMARY KEY,
lower_bound REAL NOT NULL,
upper_bound REAL NOT NULL,
stepsize REAL NOT NULL,
question_description VARCHAR(3000) NOT NULL,
approved BOOLEAN NOT NULL
);

CREATE TABLE categorical_types (
approved BOOLEAN NOT NULL,
question_description VARCHAR(3000),
comma_separated_options VARCHAR(3000),
id BIGSERIAL PRIMARY KEY
);

CREATE TABLE user_question_responses (
    id BIGSERIAL ,
    user_id BIGSERIAL REFERENCES users(id),
    response_time TIMESTAMP NOT NULL,
    numeric_type_id BIGSERIAL REFERENCES numeric_types(id) ,
    categorical_type_id BIGSERIAL REFERENCES categorical_types(id),
    is_hidden BOOLEAN, 
    PRIMARY KEY(id, user_id),
    CONSTRAINT check_only_one_not_null CHECK (num_nonnulls(numeric_type_id, categorical_type_id) = 1)
);