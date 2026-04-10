use gloo_timers::future::TimeoutFuture;
use leptos::ev::{Event, MouseEvent, SubmitEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_params_map;
use leptos_router::path;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::api::{self, ApiError};
use crate::model::{
	ApiProblem, DraftMove, GameResponse, LoginRequest, RegisterRequest, Spot, User,
};

#[derive(Debug, Clone)]
struct Toast {
	id: u64,
	title: String,
	message: String,
}

#[component]
pub fn App() -> impl IntoView {
	view! {
		<Router>
			<BackgroundStage />
			<main class="shell">
				<Routes fallback=|| view! { <NotFoundPage /> }>
					<Route path=path!("/") view=LandingPage />
					<Route path=path!("/game/:id") view=GamePage />
				</Routes>
			</main>
		</Router>
	}
}

#[component]
fn BackgroundStage() -> impl IntoView {
	view! {
		<div class="background-stage" aria-hidden="true">
			<svg viewBox="0 0 1200 900" preserveAspectRatio="none">
				<g class="bg-board bg-board--one">
					<polyline points="140,210 220,180 340,190 450,260 540,250"></polyline>
					<circle cx="140" cy="210" r="10"></circle>
					<circle cx="340" cy="190" r="10"></circle>
					<circle cx="450" cy="260" r="10"></circle>
				</g>
				<g class="bg-board bg-board--two">
					<polyline points="760,170 860,150 980,170 1030,260 980,340 860,330"></polyline>
					<circle cx="760" cy="170" r="10"></circle>
					<circle cx="980" cy="170" r="10"></circle>
					<circle cx="860" cy="330" r="10"></circle>
				</g>
				<g class="bg-board bg-board--three">
					<polyline points="240,630 330,570 470,560 590,610 650,710 560,770 410,760"></polyline>
					<circle cx="240" cy="630" r="10"></circle>
					<circle cx="470" cy="560" r="10"></circle>
					<circle cx="560" cy="770" r="10"></circle>
				</g>
			</svg>
		</div>
	}
}

#[component]
fn LandingPage() -> impl IntoView {
	let user = RwSignal::new(None::<User>);
	let toasts = RwSignal::new(Vec::<Toast>::new());
	let next_toast_id = RwSignal::new(0_u64);
	let register_username = RwSignal::new(String::new());
	let register_password = RwSignal::new(String::new());
	let login_username = RwSignal::new(String::new());
	let login_password = RwSignal::new(String::new());
	let join_game_id = RwSignal::new(String::new());

	Effect::new(move |_| {
		spawn_local(async move {
			match api::me().await {
				Ok(current_user) => user.set(current_user),
				Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
			}
		});
	});

	let on_register = move |event: SubmitEvent| {
		event.prevent_default();

		let payload = RegisterRequest {
			username: register_username.get(),
			password: register_password.get(),
		};

		spawn_local(async move {
			match api::register(&payload).await {
				Ok(()) => match api::login(&LoginRequest {
					username: payload.username,
					password: payload.password,
				})
				.await
				{
					Ok(()) => match api::me().await {
						Ok(current_user) => user.set(current_user),
						Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
					},
					Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
				},
				Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
			}
		});
	};

	let on_login = move |event: SubmitEvent| {
		event.prevent_default();

		let payload = LoginRequest {
			username: login_username.get(),
			password: login_password.get(),
		};

		spawn_local(async move {
			match api::login(&payload).await {
				Ok(()) => match api::me().await {
					Ok(current_user) => user.set(current_user),
					Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
				},
				Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
			}
		});
	};

	let on_logout = move |_| {
		spawn_local(async move {
			match api::logout().await {
				Ok(()) => user.set(None),
				Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
			}
		});
	};

	view! {
		<section class="page">
			<div class="main-panel">
				<header class="panel-bar">
					<div class="panel-bar__title">
						<h1>"Sprouts"</h1>
						<p>"Create or join a room."</p>
					</div>
					<Show when=move || user.get().is_some()>
						<div class="panel-bar__session">
							<span class="session-indicator">
								{move || {
									user.get()
										.map(|current_user| format!("Signed in as {}", current_user.username))
										.unwrap_or_default()
								}}
							</span>
							<button class="button--ghost" on:click=on_logout>
								"Logout"
							</button>
						</div>
					</Show>
				</header>

				{move || {
					if user.get().is_some() {
						view! {
							<div class="panel-body panel-body--landing">
								<section class="action-center">
									<div class="action-slot">
										<h2>"Create game"</h2>
										<p>"Start a new room with the initial two-spot board."</p>
										<button
											on:click=move |_| {
												spawn_local(async move {
													match api::create_game().await {
														Ok(game) => go_to(&format!("/game/{}", game.id())),
														Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
													}
												});
											}
										>
											"Create game"
										</button>
									</div>

									<form
										class="action-slot action-slot--join"
										on:submit=move |event: SubmitEvent| {
											event.prevent_default();
											let game_id = join_game_id.get();

											spawn_local(async move {
												match api::join_game(&game_id).await {
													Ok(game) => go_to(&format!("/game/{}", game.id())),
													Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
												}
											});
										}
									>
										<h2>"Join game"</h2>
										<p>"Enter an 8-character room code."</p>
										<label class="join-label">
											<span>"Join code"</span>
										</label>
										<div class="join-inline">
											<input
												placeholder="019D776A"
												prop:value=move || join_game_id.get()
												on:input=move |ev| join_game_id.set(event_target_value(&ev))
											/>
											<button type="submit">"Join game"</button>
										</div>
									</form>
								</section>
							</div>
						}
							.into_any()
					} else {
						view! {
							<div class="panel-body panel-body--landing">
								<div class="auth-grid">
									<form class="stack-form auth-pane" on:submit=on_register>
										<h2>"Register"</h2>
										<p>"Create a local user for this room-based MVP."</p>
										<label>
											<span>"Username"</span>
											<input
												prop:value=move || register_username.get()
												on:input=move |ev| register_username.set(event_target_value(&ev))
											/>
										</label>
										<label>
											<span>"Password"</span>
											<input
												type="password"
												prop:value=move || register_password.get()
												on:input=move |ev| register_password.set(event_target_value(&ev))
											/>
										</label>
										<button type="submit">"Create account"</button>
									</form>

									<form class="stack-form auth-pane" on:submit=on_login>
										<h2>"Login"</h2>
										<p>"Reuse an existing session-backed account."</p>
										<label>
											<span>"Username"</span>
											<input
												prop:value=move || login_username.get()
												on:input=move |ev| login_username.set(event_target_value(&ev))
											/>
										</label>
										<label>
											<span>"Password"</span>
											<input
												type="password"
												prop:value=move || login_password.get()
												on:input=move |ev| login_password.set(event_target_value(&ev))
											/>
										</label>
										<button type="submit">"Login"</button>
									</form>
								</div>
							</div>
						}
							.into_any()
					}
				}}
			</div>
			<ErrorToasts toasts=toasts />
		</section>
	}
}

#[component]
fn GamePage() -> impl IntoView {
	let params = use_params_map();
	let game = RwSignal::new(None::<GameResponse>);
	let toasts = RwSignal::new(Vec::<Toast>::new());
	let next_toast_id = RwSignal::new(0_u64);
	let current_user = RwSignal::new(None::<User>);
	let draft = RwSignal::new(DraftMove::empty());
	let hover_point = RwSignal::new(None::<[f64; 2]>);

	let game_id = move || params.read().get("id").unwrap_or_default();
	Effect::new(move |_| {
		let current_game_id = game_id();

		spawn_local(async move {
			loop {
				if current_pathname() != format!("/game/{current_game_id}") {
					break;
				}

				match api::me().await {
					Ok(Some(user)) => current_user.set(Some(user)),
					Ok(None) => {
						go_to("/");
						break;
					}
					Err(api_error) => {
						push_api_error(toasts, next_toast_id, api_error);
						break;
					}
				}

				match api::get_game(&current_game_id).await {
					Ok(next_game) => game.set(Some(next_game)),
					Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
				}

				TimeoutFuture::new(2_000).await;
			}
		});
	});

	let on_board_click = move |event: MouseEvent| {
		let Some(next_draft) = game
			.get()
			.and_then(|current_game| svg_click_point(&event).map(|point| (current_game, point)))
			.map(|(current_game, point)| {
				let mut next_draft = draft.get();
				let clicked_spot = current_game
					.board_state()
					.spots
					.iter()
					.find(|spot| point_distance(point, [spot.x, spot.y]) <= 14.0)
					.cloned();

				if let Some(spot) = clicked_spot {
					if next_draft.start_spot.is_none() {
						next_draft.start_spot = Some(spot.clone());
						next_draft.points = vec![[spot.x, spot.y]];
						hover_point.set(Some([spot.x, spot.y]));
					} else if next_draft.end_spot.is_none() {
						next_draft.end_spot = Some(spot.clone());
						next_draft.points.push([spot.x, spot.y]);
						hover_point.set(Some([spot.x, spot.y]));
					}
				} else if next_draft.start_spot.is_some() && next_draft.end_spot.is_none() {
					next_draft.points.push(point);
				} else if next_draft.end_spot.is_some() && next_draft.new_spot.is_none() {
					next_draft.new_spot = snap_point_to_polyline(point, &next_draft.points);
				}

				next_draft
			})
		else {
			return;
		};

		let should_submit = next_draft.can_submit();
		draft.set(next_draft.clone());

		if should_submit {
			submit_draft_move(game_id(), next_draft, game, draft, toasts, next_toast_id);
		}
	};

	let on_board_move = move |event: MouseEvent| {
		let hover = svg_click_point(&event).map(|point| {
			let current_draft = draft.get();

			if current_draft.end_spot.is_some() && current_draft.new_spot.is_none() {
				snap_point_to_polyline(point, &current_draft.points).unwrap_or(point)
			} else {
				point
			}
		});

		hover_point.set(hover);
	};

	let on_board_leave = move |_| hover_point.set(None);

	let on_spot_click = move |spot: Spot| {
		let mut next_draft = draft.get();

		if next_draft.start_spot.is_none() {
			next_draft.start_spot = Some(spot.clone());
			next_draft.points = vec![[spot.x, spot.y]];
			hover_point.set(Some([spot.x, spot.y]));
		} else if next_draft.end_spot.is_none() {
			next_draft.end_spot = Some(spot.clone());
			next_draft.points.push([spot.x, spot.y]);
			hover_point.set(Some([spot.x, spot.y]));
		}

		draft.set(next_draft);
	};

	let on_reset_draft = move |_| draft.set(DraftMove::empty());

	let on_submit_move = move |_| {
		if draft.get().to_request().is_none() {
			return;
		}

		submit_draft_move(game_id(), draft.get(), game, draft, toasts, next_toast_id);
	};

	view! {
		<section class="page">
			<div class="main-panel main-panel--game">
				<header class="panel-bar">
					<div class="panel-bar__title">
						<h1>"Game room"</h1>
						<p>{move || game.get().as_ref().map(status_label).unwrap_or("Loading game state")}</p>
					</div>
					<div class="panel-bar__actions">
						<A href="/" attr:class="button--ghost">
							"Back"
						</A>
					</div>
				</header>

				<Show
					when=move || game.get().is_some()
					fallback=move || {
						view! {
							<div class="panel-body">
								<section class="section-block">
									<p>"Loading board…"</p>
								</section>
							</div>
						}
					}
				>
					{move || {
						let current_game = game.get().unwrap();
						let board_state = current_game.board_state().clone();
						let draft_points = draft.get().points;
						let draft_new_spot = draft.get().new_spot;
						let current_hover_point = hover_point.get();
						let selected_start_spot_id = draft.get().start_spot.as_ref().map(|spot| spot.id);
						let selected_end_spot_id = draft.get().end_spot.as_ref().map(|spot| spot.id);
						let current_turn = turn_label(&current_game, current_user.get().as_ref());
						let players = players_label(&current_game);
						let outcome = winner_label(&current_game, current_user.get().as_ref());
						let draft_state = draft_state_lines(&draft.get());

						view! {
							<div class="panel-body panel-body--game">
								<section class="game-stage">
									<div class="game-strip">
										<div class="game-fact">
											<span>"Game id"</span>
											<strong>{current_game.id().to_string()}</strong>
										</div>
										<div class="game-fact">
											<span>"Join code"</span>
											<strong>{current_game.join_code().to_string()}</strong>
										</div>
										<div class="game-fact">
											<span>"Players"</span>
											<strong>{players}</strong>
										</div>
										<div class="game-fact">
											<span>"Turn"</span>
											<strong>{current_turn}</strong>
										</div>
										{outcome.map(|winner| {
											view! {
												<div class="game-fact">
													<span>"Winner"</span>
													<strong>{winner}</strong>
												</div>
											}
										})}
									</div>

									<svg
										id="game-board"
										class="board"
										viewBox="0 0 400 240"
										on:click=on_board_click
										on:mousemove=on_board_move
										on:mouseleave=on_board_leave
									>
										<rect x="0" y="0" width="400" height="240" class="board__surface"></rect>
										<For
											each=move || board_state.edges.clone()
											key=|edge| edge.id
											children=move |edge| {
												view! {
													<polyline class="board__edge" points=polyline_points(&edge.points)>
														<title>{format!(
															"Edge {}: {} -> {} via new spot {}",
															edge.id,
															edge.start_spot_id,
															edge.end_spot_id,
															edge.new_spot_id
														)}</title>
													</polyline>
												}
											}
										/>
										{if draft_points.len() > 1 {
											Some(view! {
												<polyline
													class="board__edge board__edge--draft"
													points=polyline_points(&draft_points)
												></polyline>
											})
										} else {
											None
										}}
										<For
											each=move || draft_points.clone()
											key=|point| format!("{:.3}-{:.3}", point[0], point[1])
											children=move |point| {
												view! {
													<circle
														class="board__draft-point"
														cx=point[0]
														cy=point[1]
														r="4"
													></circle>
												}
											}
										/>
										<For
											each=move || board_state.spots.clone()
											key=|spot| spot.id
											children=move |spot| {
												let circle_spot = spot.clone();
												let text_spot = spot.clone();
												let is_selected = selected_start_spot_id == Some(spot.id)
													|| selected_end_spot_id == Some(spot.id);
												view! {
													<g class="board__spot" class:board__spot--selected=is_selected>
														<circle
															cx=spot.x
															cy=spot.y
															r="11"
															on:click=move |event| {
																event.stop_propagation();
																on_spot_click(circle_spot.clone());
															}
														></circle>
														<title>{format!(
															"Spot {} · degree {}",
															spot.id, spot.degree
														)}</title>
														<text
															x=spot.x
															y=spot.y + 4.0
															on:click=move |event| {
																event.stop_propagation();
																on_spot_click(text_spot.clone());
															}
														>
															{spot.id}
														</text>
													</g>
												}
											}
										/>
										<Show when=move || current_hover_point.is_some() && draft_new_spot.is_none()>
											<circle
												class="board__hover-point"
												cx=move || current_hover_point.map(|point| point[0]).unwrap_or_default()
												cy=move || current_hover_point.map(|point| point[1]).unwrap_or_default()
												r="5"
											></circle>
										</Show>
										<Show when=move || draft_new_spot.is_some()>
											<circle
												class="board__new-spot"
												cx=move || draft_new_spot.map(|point| point[0]).unwrap_or_default()
												cy=move || draft_new_spot.map(|point| point[1]).unwrap_or_default()
												r="8"
											></circle>
										</Show>
									</svg>
								</section>

								<aside class="game-sidebar">
									<div class="game-sidebar__header">
										<div>
											<h2>"Draft move"</h2>
											<p>"Build the polyline in four steps and submit once."</p>
										</div>
									</div>
									<div class="draft-list">
										<p>{move || draft_summary(&draft.get())}</p>
									</div>
									<div class="draft-state">
										<For
											each=move || draft_state.clone()
											key=|line| line.clone()
											children=move |line| {
												view! { <p>{line}</p> }
											}
										/>
									</div>
									<ul class="step-list">
										<li>"1. Select start spot"</li>
										<li>"2. Add intermediate points"</li>
										<li>"3. Select end spot"</li>
										<li>"4. Place the new spot"</li>
									</ul>
									<div class="control-row">
										<button class="button--ghost" on:click=on_reset_draft>
											"Reset"
										</button>
										<button
											prop:disabled=move || !draft.get().can_submit()
											on:click=on_submit_move
										>
											"Submit move"
										</button>
									</div>
								</aside>
							</div>
						}
					}}
				</Show>

			</div>
			<ErrorToasts toasts=toasts />
		</section>
	}
}

#[component]
fn NotFoundPage() -> impl IntoView {
	view! {
		<section class="page">
			<div class="main-panel">
				<div class="panel-body">
					<section class="section-block">
						<h1>"Not found"</h1>
						<p>"That route doesn't exist yet."</p>
						<A href="/" attr:class="button--ghost">
							"Go back"
						</A>
					</section>
				</div>
			</div>
		</section>
	}
}

#[component]
fn ErrorToasts(toasts: RwSignal<Vec<Toast>>) -> impl IntoView {
	view! {
		<Show when=move || !toasts.get().is_empty()>
			<div class="error-toast-stack" aria-live="polite">
				<For
					each=move || toasts.get()
					key=|toast| toast.id
					children=move |toast| {
						let toast_id = toast.id;
						view! {
							<aside class="error-toast" role="alert">
								<div class="error-toast__content">
									<strong>{toast.title}</strong>
									<p>{toast.message}</p>
								</div>
								<button
									class="error-toast__dismiss"
									on:click=move |_| dismiss_toast(toasts, toast_id)
								>
									"Dismiss"
								</button>
							</aside>
						}
					}
				/>
			</div>
		</Show>
	}
}

fn event_target_value(event: &Event) -> String {
	event
		.target()
		.and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
		.map(|input| input.value())
		.unwrap_or_default()
}

fn polyline_points(points: &[[f64; 2]]) -> String {
	points
		.iter()
		.map(|point| format!("{},{}", point[0], point[1]))
		.collect::<Vec<_>>()
		.join(" ")
}

fn draft_summary(draft: &DraftMove) -> String {
	match (&draft.start_spot, &draft.end_spot, draft.new_spot) {
		(None, _, _) => "Select a start spot.".to_string(),
		(Some(start), None, _) => format!(
			"Start: #{}. Add path points or select the end spot.",
			start.id
		),
		(Some(start), Some(end), None) => format!(
			"Start: #{} · End: #{}. Place the new spot on the line.",
			start.id, end.id
		),
		(Some(start), Some(end), Some(_)) => {
			format!("Ready to submit a move from #{} to #{}.", start.id, end.id)
		}
	}
}

fn draft_state_lines(draft: &DraftMove) -> Vec<String> {
	vec![
		match &draft.start_spot {
			Some(spot) => format!("Start spot: #{}", spot.id),
			None => "Start spot: not selected".to_string(),
		},
		match &draft.end_spot {
			Some(spot) => format!("End spot: #{}", spot.id),
			None => "End spot: not selected".to_string(),
		},
		match draft.new_spot {
			Some([x, y]) => format!("New spot: ({x:.1}, {y:.1})"),
			None => "New spot: not placed".to_string(),
		},
	]
}

fn status_label(game: &GameResponse) -> &'static str {
	match game {
		GameResponse::Waiting { .. } => "Waiting for second player",
		GameResponse::Active { .. } => "Game active",
		GameResponse::Finished { .. } => "Game finished",
	}
}

fn players_label(game: &GameResponse) -> String {
	match game {
		GameResponse::Waiting { player1, .. } => {
			format!("{} · waiting for another player", player1.username)
		}
		GameResponse::Active {
			player1, player2, ..
		}
		| GameResponse::Finished {
			player1, player2, ..
		} => format!("{} vs {}", player1.username, player2.username),
	}
}

fn turn_label(game: &GameResponse, current_user: Option<&User>) -> String {
	match game {
		GameResponse::Waiting { .. } => "Game has not started yet".to_string(),
		GameResponse::Active {
			player1,
			player2,
			current_turn_user_id,
			..
		} => {
			let current_turn = if *current_turn_user_id == player1.id {
				&player1.username
			} else {
				&player2.username
			};

			if current_user.map(|user| user.id) == Some(*current_turn_user_id) {
				format!("Your turn · {current_turn}")
			} else {
				current_turn.to_string()
			}
		}
		GameResponse::Finished { .. } => "No turns remain".to_string(),
	}
}

fn winner_label(game: &GameResponse, current_user: Option<&User>) -> Option<String> {
	match game {
		GameResponse::Finished {
			player1,
			player2,
			winner_user_id,
			..
		} => {
			let winner = if *winner_user_id == player1.id {
				&player1.username
			} else {
				&player2.username
			};

			Some(
				if current_user.map(|user| user.id) == Some(*winner_user_id) {
					format!("You · {winner}")
				} else {
					winner.to_string()
				},
			)
		}
		_ => None,
	}
}

fn svg_click_point(event: &MouseEvent) -> Option<[f64; 2]> {
	let svg = web_sys::window()
		.and_then(|window| window.document())
		.and_then(|document| document.get_element_by_id("game-board"))
		.and_then(|element| element.dyn_into::<web_sys::SvgsvgElement>().ok())?;
	let rect = svg.get_bounding_client_rect();
	let scale_x = 400.0 / rect.width();
	let scale_y = 240.0 / rect.height();
	let x = (event.client_x() as f64 - rect.left()) * scale_x;
	let y = (event.client_y() as f64 - rect.top()) * scale_y;

	Some([x, y])
}

fn current_pathname() -> String {
	web_sys::window()
		.and_then(|window| window.location().pathname().ok())
		.unwrap_or_default()
}

fn go_to(path: &str) {
	if let Some(window) = web_sys::window() {
		let _ = window.location().set_href(path);
	}
}

fn push_toast(
	toasts: RwSignal<Vec<Toast>>,
	next_toast_id: RwSignal<u64>,
	title: String,
	message: String,
) {
	let toast_id = next_toast_id.get_untracked();
	next_toast_id.set(toast_id + 1);
	toasts.update(|items| {
		items.push(Toast {
			id: toast_id,
			title,
			message,
		})
	});

	spawn_local(async move {
		TimeoutFuture::new(4_500).await;
		dismiss_toast(toasts, toast_id);
	});
}

fn dismiss_toast(toasts: RwSignal<Vec<Toast>>, toast_id: u64) {
	toasts.update(|items| items.retain(|toast| toast.id != toast_id));
}

fn push_api_error(toasts: RwSignal<Vec<Toast>>, next_toast_id: RwSignal<u64>, api_error: ApiError) {
	let (title, message) = toast_content_for_api_error(&api_error);
	push_toast(toasts, next_toast_id, title, message);
}

fn toast_content_for_api_error(api_error: &ApiError) -> (String, String) {
	match api_error {
		ApiError::Problem(problem) => problem_toast(problem),
		ApiError::Network(error) => ("Connection failed".to_string(), error.to_string()),
		ApiError::Unexpected(error) => (
			"Unexpected response".to_string(),
			format!(
				"The backend returned status {} in an unexpected format.",
				error.status
			),
		),
	}
}

fn problem_toast(problem: &ApiProblem) -> (String, String) {
	let title = problem
		.details
		.as_ref()
		.and_then(|details| details.title.clone())
		.unwrap_or_else(|| "Request failed".to_string());
	let message = problem
		.details
		.as_ref()
		.and_then(|details| {
			details
				.detail
				.clone()
				.or_else(|| details.errors.first().map(|field| field.detail.clone()))
		})
		.unwrap_or_else(|| problem.message.clone());

	(title, message)
}

fn submit_draft_move(
	game_id: String,
	submitted_draft: DraftMove,
	game: RwSignal<Option<GameResponse>>,
	draft: RwSignal<DraftMove>,
	toasts: RwSignal<Vec<Toast>>,
	next_toast_id: RwSignal<u64>,
) {
	let Some(request) = submitted_draft.to_request() else {
		return;
	};

	spawn_local(async move {
		match api::submit_move(&game_id, &request).await {
			Ok(updated_game) => {
				game.set(Some(updated_game));
				draft.set(DraftMove::empty());
			}
			Err(api_error) => {
				handle_move_submission_error(
					&api_error,
					&game_id,
					submitted_draft,
					game,
					draft,
					toasts,
					next_toast_id,
				);
				push_api_error(toasts, next_toast_id, api_error);
			}
		}
	});
}

fn handle_move_submission_error(
	api_error: &ApiError,
	game_id: &str,
	submitted_draft: DraftMove,
	game: RwSignal<Option<GameResponse>>,
	draft: RwSignal<DraftMove>,
	toasts: RwSignal<Vec<Toast>>,
	next_toast_id: RwSignal<u64>,
) {
	let Some(problem) = (match api_error {
		ApiError::Problem(problem) => Some(problem),
		_ => None,
	}) else {
		return;
	};

	match primary_problem_code(problem) {
		Some("new_spot_not_on_path") | Some("new_spot_is_endpoint") => {
			let mut next_draft = submitted_draft;
			next_draft.new_spot = None;
			draft.set(next_draft);
		}
		Some(
			"path_self_intersects"
			| "path_intersects_existing_edge"
			| "path_touches_existing_spot"
			| "path_has_degenerate_segment"
			| "path_does_not_end_at_end_spot"
			| "spot_capacity_exceeded",
		) => {
			draft.set(reset_draft_to_start(submitted_draft));
		}
		Some("not_players_turn") | Some("game_not_active") | Some("game_not_found") => {
			draft.set(DraftMove::empty());
			refresh_game(game_id.to_owned(), game, toasts, next_toast_id);
		}
		_ => {
			draft.set(DraftMove::empty());
		}
	}
}

fn primary_problem_code(problem: &ApiProblem) -> Option<&str> {
	problem
		.details
		.as_ref()
		.and_then(|details| details.errors.first())
		.map(|field| field.code.as_str())
}

fn reset_draft_to_start(draft: DraftMove) -> DraftMove {
	let Some(start_spot) = draft.start_spot else {
		return DraftMove::empty();
	};

	DraftMove {
		start_spot: Some(start_spot.clone()),
		points: vec![[start_spot.x, start_spot.y]],
		end_spot: None,
		new_spot: None,
	}
}

fn refresh_game(
	game_id: String,
	game: RwSignal<Option<GameResponse>>,
	toasts: RwSignal<Vec<Toast>>,
	next_toast_id: RwSignal<u64>,
) {
	spawn_local(async move {
		match api::get_game(&game_id).await {
			Ok(updated_game) => game.set(Some(updated_game)),
			Err(api_error) => push_api_error(toasts, next_toast_id, api_error),
		}
	});
}

fn point_distance(left: [f64; 2], right: [f64; 2]) -> f64 {
	let dx = left[0] - right[0];
	let dy = left[1] - right[1];
	(dx * dx + dy * dy).sqrt()
}

fn snap_point_to_polyline(point: [f64; 2], polyline: &[[f64; 2]]) -> Option<[f64; 2]> {
	polyline
		.windows(2)
		.map(|segment| closest_point_on_segment(point, segment[0], segment[1]))
		.min_by(|left, right| {
			point_distance(point, *left)
				.partial_cmp(&point_distance(point, *right))
				.unwrap_or(std::cmp::Ordering::Equal)
		})
}

fn closest_point_on_segment(point: [f64; 2], start: [f64; 2], end: [f64; 2]) -> [f64; 2] {
	let dx = end[0] - start[0];
	let dy = end[1] - start[1];
	let length_squared = dx * dx + dy * dy;

	if length_squared == 0.0 {
		return start;
	}

	let projection = ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / length_squared;
	let t = projection.clamp(0.0, 1.0);

	[start[0] + t * dx, start[1] + t * dy]
}
