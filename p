diff --git a/frontend/src/app.rs b/frontend/src/app.rs
index 62bd777..89b3799 100644
--- a/frontend/src/app.rs
+++ b/frontend/src/app.rs
@@ -12,10 +12,7 @@ use yewtil::{
 
 use shared::*;
 
-use crate::components::{
-    chat::Chat, chatlist::Chatlist, messages::Props as MessagesProps, modal::Modal,
-    sidebar::Sidebar,
-};
+use crate::components::{chat::Chat, chatlist::Chatlist, windowmanager::{FileManager, Props as FileManagerProps}, messages::Props as MessagesProps, modal::Modal, sidebar::Sidebar};
 
 #[derive(Debug)]
 pub enum WsAction {
@@ -148,7 +145,6 @@ impl App {
                     chat_id,
                 })
             });
-
             let messages_props = props! {
                 MessagesProps {
                     messages: self.model.messages.irc(),
@@ -156,6 +152,7 @@ impl App {
                     messages_range: self.model.messages_range.irc(),
                     selected_chat_id: self.model.selected_chat_id.irc(),
                     fetch_callback: messages_fetch_callback,
+
                 }
             };
 
@@ -174,35 +171,46 @@ impl App {
             }
         };
 
+
+        let file_manager_props = props! {
+            FileManagerProps {
+                left: html! {
+                    <>
+                    <Sidebar
+                    accounts=self.model.accounts.irc()
+                    selected_account=self.model.selected_account.irc()
+                    select_account_callback=select_account_callback
+                    create_account_callback=create_account_callback
+                  />
+                  <Chatlist
+                    selected_account=self.model.selected_account.irc()
+                    selected_account_details=account_details
+                    selected_chat_id=self.model.selected_chat_id.irc()
+                    selected_chat=self.model.selected_chat.irc()
+                    selected_chat_length=self.model.selected_chat_length.irc()
+                    select_chat_callback=select_chat_callback
+                    pin_chat_callback=pin_chat_callback
+                    unpin_chat_callback=unpin_chat_callback
+                    archive_chat_callback=archive_chat_callback
+                    unarchive_chat_callback=unarchive_chat_callback
+                    chats=self.model.chats.irc()
+                    chats_range=self.model.chats_range.irc()
+                    chats_len=self.model.chats_len.irc()
+                    fetch_callback=chats_fetch_callback />
+                    </>
+                },
+                center: html! {
+                    {{messages}}
+                },
+                right: None
+            }
+        };
+
         html! {
             <>
-            { account_creation_modal }
-              <div class="app">
-                <Sidebar
-                  accounts=self.model.accounts.irc()
-                  selected_account=self.model.selected_account.irc()
-                  select_account_callback=select_account_callback
-                  create_account_callback=create_account_callback
-                />
-                <Chatlist
-                  selected_account=self.model.selected_account.irc()
-                  selected_account_details=account_details
-                  selected_chat_id=self.model.selected_chat_id.irc()
-                  selected_chat=self.model.selected_chat.irc()
-                  selected_chat_length=self.model.selected_chat_length.irc()
-                  select_chat_callback=select_chat_callback
-                  pin_chat_callback=pin_chat_callback
-                  unpin_chat_callback=unpin_chat_callback
-                  archive_chat_callback=archive_chat_callback
-                  unarchive_chat_callback=unarchive_chat_callback
-                  chats=self.model.chats.irc()
-                  chats_range=self.model.chats_range.irc()
-                  chats_len=self.model.chats_len.irc()
-                  fetch_callback=chats_fetch_callback />
-
-                {{messages}}
-            </div>
-           </>
+                {account_creation_modal}
+                <FileManager with file_manager_props/>
+            </>
         }
     }
 }
diff --git a/frontend/src/components/mod.rs b/frontend/src/components/mod.rs
index c5d16d6..2b3e16d 100644
--- a/frontend/src/components/mod.rs
+++ b/frontend/src/components/mod.rs
@@ -9,3 +9,4 @@ pub mod modal;
 
 pub mod chat;
 pub mod context_menu;
+pub mod windowmanager;
\ No newline at end of file
diff --git a/frontend/src/style.scss b/frontend/src/style.scss
index c7d1568..a027c47 100644
--- a/frontend/src/style.scss
+++ b/frontend/src/style.scss
@@ -20,16 +20,9 @@ body {
 @import "./styles/modal";
 @import "./styles/account-create";
 @import "./styles/context-menu";
+@import "./styles/windowmanager.scss";
 
 // Layout
-
-.app {
-  display: flex;
-  flex-direction: row;
-  min-height: 100vh;
-  height: 100%;
-}
-
 .account-header {
   flex: 0 0 50px;
   display: flex;
