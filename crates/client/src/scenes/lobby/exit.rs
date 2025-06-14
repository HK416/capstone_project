use chrono::Local;
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::RawPacket;
use mod_render::UiRenderer;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{TexturePool, TextureViewPool, ARONA_SAD_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["게임 종료"];
/// 애플리케이션 표시 언어에 따른 메시지 텍스트입니다.
const MESSAGE_TEXTS: [&'static str; NUM_LOCALE] = ["선생님 벌써 가시는 건가요...?"];
/// 애플리케이션 표시 언어에 따른 `확인` 버튼 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["응"];
/// 애플리케이션 표시 언어에 따른 `취소` 버튼 텍스트입니다.
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["아니"];

/// 게임의 메인 로비 화면입니다.
/// 게임 종료를 확인하기 위한 모달 대화 상자를 화면에 표시합니다.
pub struct MainLobbyExitModalScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// 확인 버튼 상태
    okay_btn_state: ButtonState,
    /// 취소 버튼 상태
    cancel_btn_state: ButtonState,
    /// 입력 지연 시간
    delay_time_sec: f32,

    /// 아로나 이미지 텍스처
    arona_img_texture: egui::load::SizedTexture,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl MainLobbyExitModalScene {
    /// 새로운 `MainLobbyExitModalScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
    ) -> Self {
        Self {
            locale,
            okay_btn_state: ButtonState::Idle,
            cancel_btn_state: ButtonState::Idle,
            delay_time_sec: 0.3,
            arona_img_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            texture_pool,
            texture_view_pool,
        }
    }

    /// Ui 렌더러에 텍스처를 등록합니다.
    fn regist_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 아로나 이미지 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(ARONA_SAD_URI)
            .expect("Arona_Sad texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 아로나 이미지 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.arona_img_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// Ui 렌더러에 등록된 텍스처를 해제합니다.
    fn unregist_texture(&mut self, ui_renderer: &mut UiRenderer) {
        ui_renderer.free_texture(&self.arona_img_texture.id);
    }
}

impl GameScene for MainLobbyExitModalScene {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_texture(device, ui_renderer);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        self.unregist_texture(ui_renderer);
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        Some(packet)
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !repeat && self.delay_time_sec <= 0.0 {
            match code {
                KeyCode::Escape => {
                    // 게임 장면에서 빠져나옵니다.
                    let scene_flow = GameSceneFlow::Pop;
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
                KeyCode::Enter => {
                    // 모든 게임 장면을 제거합니다.
                    let scene_flow = GameSceneFlow::Clear;
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
                _ => {}
            }
        }

        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0)
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let locale = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 타이틀 텍스트
        let text = TITLE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 메시지 텍스트
        let text = MESSAGE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let message_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `확인` 버튼 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `취소` 버튼 텍스트
        let text = CANCEL_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `예` 버튼
        let fill = match self.okay_btn_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let okay_button = egui::Button::new(okay_text)
            .sense(egui::Sense::all())
            .fill(fill)
            .corner_radius(3.0)
            .min_size((180.0 * scale, 45.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        // `아니오` 버튼
        let fill = match self.cancel_btn_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let cancel_button = egui::Button::new(cancel_text)
            .sense(egui::Sense::all())
            .fill(fill)
            .corner_radius(3.0)
            .min_size((180.0 * scale, 45.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(5.0 * scale)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Exit_Onemore"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96))
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(640.0 * scale);
                ui.set_max_width(640.0 * scale);

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.label(title_text);
                    ui.separator();

                    let image = egui::Image::new(self.arona_img_texture)
                        .max_size(egui::Vec2::splat(360.0) * scale);
                    ui.add(image);

                    ui.add_space(8.0 * scale);
                    ui.label(message_text);
                    ui.add_space(16.0 * scale);

                    let enable = self.okay_btn_state != ButtonState::Clicked
                        && self.cancel_btn_state != ButtonState::Clicked;
                    ui.add_enabled_ui(enable, |ui| {
                        egui::Grid::new(egui::Id::new("Button_Grid"))
                            .min_col_width(640.0 * 0.5 * scale)
                            .max_col_width(640.0 * 0.5 * scale)
                            .show(ui, |ui| {
                                ui.set_max_height(45.0 * scale);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // 예 버튼
                                        let response = ui.add(okay_button);
                                        if response.clicked() && self.delay_time_sec <= 0.0 {
                                            self.okay_btn_state = ButtonState::Clicked;

                                            // 모든 게임 장면을 제거합니다.
                                            let scene_flow = GameSceneFlow::Clear;
                                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                                            let event_loop_proxy = app.event_loop_proxy();
                                            event_loop_proxy.send_event(event).unwrap();
                                        } else if response.is_pointer_button_down_on() {
                                            self.okay_btn_state = ButtonState::Pressed;
                                        } else if response.hovered() | response.has_focus() {
                                            self.okay_btn_state = ButtonState::Hovered;
                                        } else {
                                            self.okay_btn_state = ButtonState::Idle;
                                        }
                                    },
                                );

                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        // 취소 버튼
                                        let response = ui.add(cancel_button);
                                        if response.clicked() && self.delay_time_sec <= 0.0 {
                                            self.cancel_btn_state = ButtonState::Clicked;

                                            // 게임 장면을 전환합니다.
                                            let scene_flow = GameSceneFlow::Pop;
                                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                                            let event_loop_proxy = app.event_loop_proxy();
                                            event_loop_proxy.send_event(event).unwrap();
                                        } else if response.is_pointer_button_down_on() {
                                            self.cancel_btn_state = ButtonState::Pressed;
                                        } else if response.hovered() | response.has_focus() {
                                            self.cancel_btn_state = ButtonState::Hovered;
                                        } else {
                                            self.cancel_btn_state = ButtonState::Idle;
                                        }
                                    },
                                );
                            });
                    });
                });
                ui.add_space(18.0 * scale);
            });
    }
}
