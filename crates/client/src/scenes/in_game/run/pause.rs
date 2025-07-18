use std::{ptr::NonNull, time::Instant};

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::{PacketType, RawPacket};
use mod_render::UiRenderer;
use rodio::Sink;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{SoundDataPool, NOTOSANS_BOLD, NOTOSANS_REGULAR, UI_NOTICE},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, InGameRunScene, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, NEG_COLOR, NEG_FOCUS_COLOR,
    },
};

/// 애플리케이션 표시 언어에 따른 대화 상자 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["일시 정지"];

/// 애플리케이션 표시 언어에 따른 `계속 게임하기 버튼` 텍스트입니다.
const RESUME_TEXTS: [&'static str; NUM_LOCALE] = ["계속 하기"];

/// 인게임에서 일시정지 대화상자를 출력하는 게임 장면입니다.
pub struct InGamePauseLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// `InGameRunScene`의 포인터입니다.
    ///
    /// # Safety
    /// `InGameRunScene`이 해제될 경우 정의되지 않은 동작을 수행합니다.
    ///
    in_game_scene: NonNull<InGameRunScene>,

    /// 계속 하기 버튼 상태
    resume_btn_state: ButtonState,

    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InGamePauseLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new(
        locale: Locale,
        in_game_scene: NonNull<InGameRunScene>,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            in_game_scene,
            resume_btn_state: ButtonState::Idle,
            sound_data_pool,
        }
    }
}

impl GameScene for InGamePauseLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn should_update_subscene(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let event = AppEvent::CursorDisable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let scene = unsafe { self.in_game_scene.as_ref() };

        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            scene.get_background_volume(),
            scene.get_effect_volume(),
            scene.get_voice_volume(),
            title,
            message,
            self.sound_data_pool.clone(),
        );
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 효과음을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(UI_NOTICE)
            .expect("UI_Notice sound must be preloaded!");
        let source = decoded.as_source();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(scene.get_effect_volume() as f32 / 255.0);
        sink.append(source);
        sink.play();
        sink.detach();
    }

    fn on_received_packet(
        &mut self,
        _time_stamp: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        if packet_type == PacketType::InGameFinish {
            // 현재 장면에서 빠져나옵니다.
            let flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

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
        if !repeat && code == KeyCode::Escape {
            // 이전 게임 장면으로 되돌아갑니다.
            let scene_flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        true
    }

    fn ui_callback(&mut self, _window: &Window, app: &dyn AppHandle) {
        let scene = unsafe { self.in_game_scene.as_ref() };
        let clip_rect = scene.get_clip_rect();
        let scale = scene.get_ui_scale();
        let i = self.locale as usize;

        let ctx = app.egui_ctx();
        let modal_width = clip_rect.width() * 0.25;

        // 타이틀 텍스트
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let text = egui::RichText::new(TITLE_TEXTS[i])
            .font(font_id)
            .color(FONT_COLOR);
        let title = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 계속하기 버튼 텍스트
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(18.0 * scale, family);
        let text = egui::RichText::new(RESUME_TEXTS[i])
            .font(font_id)
            .color(FONT_COLOR);

        // 계속하기 버튼
        let (bg_color, line_color) = match self.resume_btn_state {
            ButtonState::Idle => (NEG_COLOR, egui::Color32::TRANSPARENT),
            ButtonState::Hovered => (NEG_COLOR, NEG_FOCUS_COLOR),
            ButtonState::Pressed | ButtonState::Clicked => (NEG_FOCUS_COLOR, NEG_FOCUS_COLOR),
        };
        let resume_button = egui::Button::new(text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((modal_width * 0.9, modal_width * 0.9 * 0.2).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(20.0 * scale)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Pause_Modal"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96))
            .show(ctx, |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(modal_width);
                ui.set_max_width(modal_width);

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.add(title);
                    ui.separator();

                    ui.add_space(8.0 * scale);
                    // 계속 하기 버튼
                    let response = ui.add(resume_button);
                    if response.clicked() {
                        self.resume_btn_state = ButtonState::Clicked;

                        // 이전 게임 장면으로 되돌아갑니다.
                        let scene_flow = GameSceneFlow::Pop;
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    } else if response.is_pointer_button_down_on() {
                        self.resume_btn_state = ButtonState::Pressed;
                    } else if response.hovered() | response.has_focus() {
                        self.resume_btn_state = ButtonState::Hovered;
                    } else {
                        self.resume_btn_state = ButtonState::Idle;
                    }
                    ui.add_space(8.0 * scale);
                });
            });
    }
}

unsafe impl Send for InGamePauseLayer {}
