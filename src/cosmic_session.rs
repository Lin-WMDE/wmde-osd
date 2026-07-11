// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use zbus::proxy;

#[proxy(
    interface = "fun.wmde.Session",
    default_service = "fun.wmde.Session",
    default_path = "/fun/wmde/Session"
)]
pub trait CosmicSession {
    fn exit(&self) -> zbus::Result<()>;
}
