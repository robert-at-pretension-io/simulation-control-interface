CREATE TABLE numeric_types (
start DOUBLE PRECISION NOT NULL,
end DOUBLE PRECISION NOT NULL,
stepsize DOUBLE PRECISION NOT NULL,
question_description VARCHAR(3000) NOT NULL,
id BIGSERIAL PRIMARY KEY
);

CREATE TABLE categorical_types (
question_description VARCHAR(3000),
comma_separated_options VARCHAR(3000),
id BIGSERIAL PRIMARY KEY
);

CREATE TABLE user_question_responses (
    user_id NOT NULL REFERENCES users(id),
    numeric_type_id REFERENCES numeric_types(id),
    categorical_type_id REFERENCES categorical_types(id)
    is_hidden BOOLEAN,
);