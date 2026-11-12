//! `/api/measure`: the measure job's state (GET) and starting it (POST, behind the [`Guard`]).

use super::guard::Guard;
use crate::AppState;
use axum::Json;
use axum::extract::State;
use brickie_web_api::*;

pub async fn status(State(st): State<AppState>) -> Json<ResOf<MeasureStatus>> {
    Json(st.measure.status())
}

pub async fn start(State(st): State<AppState>, _: Guard) -> Json<ResOf<MeasureStart>> {
    Json(st.measure.start(st.config()))
}
