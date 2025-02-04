use crate::{http_server::pages::blog::md::SyntaxHighlightingContext, *};

pub(crate) async fn serve() -> Result<()> {
    let app_config = AppConfig::from_env()?;
    let twitch_config = TwitchConfig::from_env()?;
    let github_config = GithubConfig::from_env()?;
    let open_ai_config = OpenAiConfig::from_env()?;
    let markdown_to_html_context = SyntaxHighlightingContext::default();

    let database_url: String = std::env::var("DATABASE_URL").or_else(|_| -> Result<String> {
        let path = std::env::var("DATABASE_PATH");

        Ok(if let Ok(p) = &path {
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(p)
                .into_diagnostic()?;

            format!("sqlite:{}", p)
        } else {
            "sqlite::memory:".to_string()
        })
    })?;

    let pool = SqlitePool::connect(&database_url).await.into_diagnostic()?;

    let blog_posts = BlogPosts::from_static_dir()?;
    let blog_posts = Arc::new(blog_posts);

    let til_posts = TilPosts::from_static_dir()?;
    let til_posts = Arc::new(til_posts);

    let streams = PastStreams::from_static_dir()?;
    let streams = Arc::new(streams);

    let app_state = AppState {
        twitch: twitch_config,
        db_pool: pool,
        github: github_config,
        app: app_config,
        open_ai: open_ai_config,
        markdown_to_html_context,
        blog_posts,
        til_posts,
        streams,
    };

    info!("About to run migrations (if any to apply)");
    migrate!("./migrations/")
        .run(&app_state.db_pool)
        .await
        .into_diagnostic()?;

    let discord_bot = build_discord_bot(app_state.clone()).await?;

    info!("Spawning Tasks");
    let discord_future = tokio::spawn(discord_bot.start());
    let axum_future = tokio::spawn(run_axum(app_state.clone()));
    info!("Tasks Spawned");

    let (discord_result, axum_result) = try_join!(discord_future, axum_future).into_diagnostic()?;

    discord_result.into_diagnostic()?;
    axum_result?;

    info!("Main Returning");

    Ok(())
}
