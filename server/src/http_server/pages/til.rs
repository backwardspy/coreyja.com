use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use maud::{html, Markup};
use miette::Result;
use reqwest::StatusCode;
use tracing::instrument;

use crate::{
    http_server::{
        pages::blog::md::IntoHtml,
        templates::{base_constrained, posts::TilPostList},
    },
    posts::til::TilPosts,
    AppConfig,
};

use super::blog::{md::SyntaxHighlightingContext, MyChannel};

#[instrument(skip_all)]
pub(crate) async fn til_index(
    State(til_posts): State<Arc<TilPosts>>,
) -> Result<Markup, StatusCode> {
    let posts = til_posts.by_recency();

    Ok(base_constrained(html! {
      h1 class="text-3xl" { "Today I Learned" }
      (TilPostList(posts))
    }))
}

#[instrument(skip_all)]
pub(crate) async fn rss_feed(
    State(config): State<AppConfig>,
    State(context): State<SyntaxHighlightingContext>,
    State(posts): State<Arc<TilPosts>>,
) -> Result<impl IntoResponse, StatusCode> {
    let channel = MyChannel::from_posts(config, context, &posts.by_recency());

    Ok(channel.into_response())
}

#[instrument(skip(til_posts, context))]
pub(crate) async fn til_get(
    State(til_posts): State<Arc<TilPosts>>,
    State(context): State<SyntaxHighlightingContext>,
    Path(slug): Path<String>,
) -> Result<Markup, StatusCode> {
    let tils = &til_posts.posts;

    let til = tils
        .iter()
        .find(|p| p.frontmatter.slug == slug)
        .ok_or(StatusCode::NOT_FOUND)?;

    let markdown = til.markdown();
    Ok(base_constrained(html! {
      h1 class="text-2xl" { (markdown.title) }
      subtitle class="block text-lg text-subtitle mb-8 " { (markdown.date) }

      div {
        (markdown.ast.into_html(&context))
      }
    }))
}
