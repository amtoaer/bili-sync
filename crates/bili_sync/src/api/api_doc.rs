use utoipa::OpenApi;
use crate::api::auth::OpenAPIAuth;

use crate::api::handlers::video_handler::{ 
    __path_get_video_sources,
    __path_get_videos, 
    __path_get_video,
    __path_reset_video 
    };
use crate::api::handlers::source_collections_handler::{ 
    __path_get_source_collections,
    __path_create_source_collection,
    __path_update_source_collection,
    __path_delete_source_collection 
    };
use crate::api::handlers::source_favorites_handler::{
    __path_get_source_favorites,
    __path_create_source_favorite,
    __path_update_source_favorite, 
    __path_delete_source_favorite 
    };
use crate::api::handlers::source_submissions_handler::{
        __path_get_source_submissions,
        __path_create_source_submission,
        __path_update_source_submission,
        __path_delete_source_submission,
    };
use crate::api::handlers::source_watch_later_handler::{
    __path_get_source_watch_later,
    __path_create_source_watch_later,
    __path_update_source_watch_later,
    __path_delete_source_watch_later,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_video_sources,
        get_videos,
        get_video,
        reset_video,

        get_source_collections,
        create_source_collection,
        update_source_collection,
        delete_source_collection,

        get_source_favorites,
        create_source_favorite,
        update_source_favorite,
        delete_source_favorite,

        get_source_submissions,
        create_source_submission,
        update_source_submission,
        delete_source_submission,

        get_source_watch_later,
        create_source_watch_later,
        update_source_watch_later,
        delete_source_watch_later,
    ),
    modifiers(&OpenAPIAuth),
    security(
        ("Token" = []),
    )
)]
pub struct ApiDoc;