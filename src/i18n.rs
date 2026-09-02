/// User-facing strings for ANUBIS Voice Teacher.
pub struct Strings {
    // Core
    pub welcome: &'static str,
    pub help: &'static str,
    pub choose_lang: &'static str,
    pub lang_set: &'static str,
    pub voice_set: &'static str,
    pub tts_fail: &'static str,
    pub ask_usage: &'static str,
    pub speak_usage: &'static str,
    pub brain_off: &'static str,
    pub reset_done: &'static str,
    pub no_voice_data: &'static str,
    pub unknown_command: &'static str,

    // Teacher mode
    pub teacher_on: &'static str,
    pub teacher_off: &'static str,
    pub teacher_status_on: &'static str,
    pub teacher_status_off: &'static str,
    pub teacher_usage: &'static str,

    // Menu / buttons
    pub menu_header: &'static str,
    pub btn_ask: &'static str,
    pub btn_speak: &'static str,
    pub btn_voices: &'static str,
    pub btn_lang: &'static str,
    pub btn_teacher: &'static str,
    pub btn_help: &'static str,
    pub btn_reset: &'static str,

    // Loading
    pub loading_think: &'static str,

    // Voice gallery
    pub voices_header: &'static str,
    pub listen_hint: &'static str,
}

pub fn get(lang: &str) -> &'static Strings {
    match lang {
        "ar" => &AR,
        "it" => &IT,
        "fr" => &FR,
        "es" => &ES,
        "de" => &DE,
        "ru" => &RU,
        "hi" => &HI,
        "tr" => &TR,
        "pt" => &PT,
        _ => &EN,
    }
}

static EN: Strings = Strings {
    welcome: "🎓 <b>ANUBIS Voice Teacher</b>\n\nYour multilingual AI language teacher.\n\n• Ask questions in any language\n• Hear your teacher speak back\n• Learn English, Arabic, Italian + more\n• Teacher mode: exercises, feedback, Socratic dialogue\n\n👇 Pick your language to begin:",
    help: "<b>Commands</b>\n\n/ask <i>[text]</i> — Ask your teacher\n/speak <i>[text]</i> — Hear speech\n/voices — Choose a voice\n/lang — Change language\n/teacher <i>on|off</i> — Toggle teacher mode\n/reset — Reset conversation\n\n💡 Or just tap a button below!",
    choose_lang: "🌐 Select your language:",
    lang_set: "✅ Language updated.",
    voice_set: "✅ Voice set.",
    tts_fail: "❌ Speech generation failed. Try again.",
    ask_usage: "Usage: /ask <your question>",
    speak_usage: "Usage: /speak <text to speak>",
    brain_off: "🧠 AI brain not configured (set ANUBIS_LLM_URL).",
    reset_done: "🧠 Conversation memory cleared.",
    no_voice_data: "ℹ️ No last reply to speak. Ask something first with /ask.",
    unknown_command: "❓ <b>Unknown command.</b> Here's what I can do:",

    teacher_on: "🎓 Teacher mode <b>enabled</b>. I'll teach you step by step.",
    teacher_off: "🎓 Teacher mode <b>disabled</b>. Back to normal assistant.",
    teacher_status_on: "🎓 Teacher mode: <b>ON</b>",
    teacher_status_off: "🎓 Teacher mode: <b>OFF</b>",
    teacher_usage: "Usage: /teacher on|off",

    menu_header: "🎓 <b>ANUBIS — Voice Teacher</b>\nChoose an action:",
    btn_ask: "🧠 Ask Teacher",
    btn_speak: "🔊 Speak",
    btn_voices: "🎙 Voices",
    btn_lang: "🌐 Language",
    btn_teacher: "🎓 Teacher Mode",
    btn_help: "❓ Help",
    btn_reset: "🔄 Reset",

    loading_think: "🧠 Thinking…",

    voices_header: "🎙 <b>Available Voices</b>\n\nTap a voice to select it as your teacher's voice:",
    listen_hint: "🔊 Tap a voice to hear it",
};

static AR: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — معلم الصوت</b>\n\nمعلمك الذكي متعدد اللغات.\n\n• اسأل أسئلة بأي لغة\n• اسمع المعلم يتكلم\n• تعلم الإنجليزية والعربية والإيطالية والمزيد\n• وضع المعلم: تمارين، ملاحظات، حوار سقراطي\n\n👇 اختر لغتك للبدء:",
    help: "<b>الأوامر</b>\n\n/ask <i>[<سؤال>]</i> — اسأل المعلم\n/speak <i>[<نص>]</i> — اسمع كلاماً\n/voices — اختر صوتاً\n/lang — غيّر اللغة\n/teacher <i>[on|off]</i> — تبديل وضع المعلم\n/reset — مسح المحادثة\n\n💡 أو اضغط الأزرار أدناه!",
    choose_lang: "🌐 اختر لغتك:",
    lang_set: "✅ تم تحديث اللغة.",
    voice_set: "✅ تم تعيين الصوت.",
    tts_fail: "❌ فشل توليد الكلام. حاول مرة أخرى.",
    ask_usage: "الاستخدام: /ask <سؤالك>",
    speak_usage: "الاستخدام: /speak <النص>",
    brain_off: "🧠 الذكاء الاصطناعي غير مهيأ.",
    reset_done: "🧠 تم مسح ذاكرة المحادثة.",
    no_voice_data: "ℹ️ لا يوجد رد لتشغيله. اسأل بشيء أولاً.",

    teacher_on: "🎓 تم تفعيل وضع المعلم <b>شغّال</b>. سأعلمك خطوة بخطوة.",
    teacher_off: "🎓 تم تعطيل وضع المعلم <b>مطفأ</b>. عاد المساعد.",
    teacher_status_on: "🎓 وضع المعلم: <b>شغّال</b>",
    teacher_status_off: "🎓 وضع المعلم: <b>مطفأ</b>",
    teacher_usage: "الاستخدام: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — معلم الصوت</b>\nاختر إجراءً:",
    btn_ask: "🧠 اسأل المعلم",
    btn_speak: "🔊 تحدث",
    btn_voices: "🎙 الأصوات",
    btn_lang: "🌐 اللغة",
    btn_teacher: "🎓 وضع المعلم",
    btn_help: "❓ مساعدة",
    btn_reset: "🔄 مسح",

    loading_think: "🧠 يفكر…",

    voices_header: "🎙 <b>الأصوات المتاحة</b>\n\nاضغط على صوت لاختياره كصوت المعلم:",
    listen_hint: "🔊 اضغط على صوت لسماعه",
    unknown_command: "❓ <b>أمر غير معروف.</b> إليك ما يمكنني فعله:",
};

static IT: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Insegnante Vocale</b>\n\nIl tuo insegnante di lingue AI multilingue.\n\n• Fai domande in qualsiasi lingua\n• Ascolta l'insegnante parlare\n• Impara inglese, arabo, italiano e altro\n• Modalità insegnante: esercizi, feedback, dialogo socratico\n\n👇 Scegli la tua lingua per iniziare:",
    help: "<b>Comandi</b>\n\n/ask <i>[<testo>]</i> — Chiedi all'insegnante\n/speak <i>[<testo>]</i> — Ascolta un parlato\n/voices — Scegli una voce\n/lang — Cambia lingua\n/teacher <i>[on|off]</i> — Attiva/disattiva modalità insegnante\n/reset — Resetta conversazione\n\n💡 O tocca un pulsante qui sotto!",
    choose_lang: "🌐 Seleziona la lingua:",
    lang_set: "✅ Lingua aggiornata.",
    voice_set: "✅ Voce impostata.",
    tts_fail: "❌ Generazione vocale fallita. Riprova.",
    ask_usage: "Uso: /ask <la tua domanda>",
    speak_usage: "Uso: /speak <testo da leggere>",
    brain_off: "🧠 AI non configurata.",
    reset_done: "🧠 Memoria conversazione azzerata.",
    no_voice_data: "ℹ️ Nessuna risposta da leggere. Fai una domanda con /ask.",

    teacher_on: "🎓 Modalità insegnante <b>attivata</b>. Ti guido passo passo.",
    teacher_off: "🎓 Modalità insegnante <b>disattivata</b>. Torno all'assistente.",
    teacher_status_on: "🎓 Modalità insegnante: <b>ON</b>",
    teacher_status_off: "🎓 Modalità insegnante: <b>OFF</b>",
    teacher_usage: "Uso: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Insegnante Vocale</b>\nScegli un'azione:",
    btn_ask: "🧠 Chiedi",
    btn_speak: "🔊 Parla",
    btn_voices: "🎙 Voci",
    btn_lang: "🌐 Lingua",
    btn_teacher: "🎓 Insegnante",
    btn_help: "❓ Aiuto",
    btn_reset: "🔄 Reset",

    loading_think: "🧠 Sto pensando…",

    voices_header: "🎙 <b>Voci Disponibili</b>\n\nTocca una voce per impostarla come voce dell'insegnante:",
    listen_hint: "🔊 Tocca una voce per ascoltarla",
    unknown_command: "❓ <b>Comando sconosciuto.</b> Ecco cosa posso fare:",
};

static FR: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Professeur Vocal</b>\n\nTon professeur de langues IA multilingue.\n\n• Pose des questions dans n'importe quelle langue\n• Écoute ton professeur parler\n• Apprends anglais, arabe, italien et plus\n• Mode prof: exercices, corrections, dialogue socratique\n\n👇 Choisis ta langue pour commencer :",
    help: "<b>Commandes</b>\n\n/ask <i>[<texte>]</i> — Demande au prof\n/speak <i>[<texte>]</i> — Écute la parole\n/voices — Choisis une voce\n/lang — Change de langue\n/teacher <i>[on|off]</i> — Active le mode prof\n/reset — Réinitialise la conversation\n\n💡 Ou touche un bouton ci-dessous !",
    choose_lang: "🌐 Choisissez votre langue :",
    lang_set: "✅ Langue mise à jour.",
    voice_set: "✅ Voix définie.",
    tts_fail: "❌ Échec de la synthèse. Réessayez.",
    ask_usage: "Usage : /ask <votre question>",
    speak_usage: "Usage : /speak <texte à dire>",
    brain_off: "🧠 IA non configurée.",
    reset_done: "🧠 Mémoire effacée.",
    no_voice_data: "ℹ️ Aucune réponse à lire. Demandez quelque chose avec /ask.",

    teacher_on: "🎓 Mode prof <b>activé</b>. Je t'enseigne pas à pas.",
    teacher_off: "🎓 Mode prof <b>désactivé</b>. Retour à l'assistant.",
    teacher_status_on: "🎓 Mode prof : <b>ON</b>",
    teacher_status_off: "🎓 Mode prof : <b>OFF</b>",
    teacher_usage: "Usage : <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Professeur Vocal</b>\nChoisissez :",
    btn_ask: "🧠 Demander",
    btn_speak: "🔊 Parler",
    btn_voices: "🎙 Voix",
    btn_lang: "🌐 Langue",
    btn_teacher: "🎓 Professeur",
    btn_help: "❓ Aide",
    btn_reset: "🔄 Reset",

    loading_think: "🧠 Je réfléchis…",

    voices_header: "🎙 <b>Voix Disponibles</b>\n\nTouchez une voix pour la définir :",
    listen_hint: "🔊 Touchez une voix pour l'écouter",
    unknown_command: "❓ <b>Commande inconnue.</b> Voici ce que je peux faire:",
};

static ES: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Profesor de Voz</b>\n\nTu profesor de idiomas IA multilingüe.\n\n• Pregunta en cualquier idioma\n• Escucha a tu profesor hablar\n• Aprende inglés, árabe, italiano y más\n• Modo profesor: ejercicios, retroalimentación, diálogo socrático\n\n👇 Elige tu idioma para empezar:",
    help: "<b>Comandos</b>\n\n/ask <i>[<texto>]</i> — Pregúntale al profesor\n/speak <i>[<texto>]</i> — Escucha el habla\n/voices — Elige una voz\n/lang — Cambia idioma\n/teacher <i>[on|off]</i> — Activa modo profesor\n/reset — Reinicia conversación\n\n💡 ¡O toca un botón abajo!",
    choose_lang: "🌐 Selecciona tu idioma:",
    lang_set: "✅ Idioma actualizado.",
    voice_set: "✅ Voz establecida.",
    tts_fail: "❌ Error al generar voz. Intenta de nuevo.",
    ask_usage: "Uso: /ask <tu pregunta>",
    speak_usage: "Uso: /speak <texto a decir>",
    brain_off: "🧠 IA no configurada.",
    reset_done: "🧠 Memoria borrada.",
    no_voice_data: "ℹ️ Sin respuesta para escuchar. Pregunta algo con /ask.",

    teacher_on: "🎓 Modo profesor <b>activado</b>. Te enseño paso a paso.",
    teacher_off: "🎓 Modo profesor <b>desactivado</b>. De vuelta al asistente.",
    teacher_status_on: "🎓 Modo profesor: <b>ON</b>",
    teacher_status_off: "🎓 Modo profesor: <b>OFF</b>",
    teacher_usage: "Uso: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Profesor de Voz</b>\nElige:",
    btn_ask: "🧠 Preguntar",
    btn_speak: "🔊 Hablar",
    btn_voices: "🎙 Voces",
    btn_lang: "🌐 Idioma",
    btn_teacher: "🎓 Profesor",
    btn_help: "❓ Ayuda",
    btn_reset: "🔄 Reset",

    loading_think: "🧠 Pensando…",

    voices_header: "🎙 <b>Voces Disponibles</b>\n\nToca una voz para seleccionarla:",
    listen_hint: "🔊 Toca una voz para escucharla",
    unknown_command: "❓ <b>Comando desconocido.</b> Esto es lo que puedo hacer:",
};

static DE: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Sprachlehrer</b>\n\nDein mehrsprachiger KI-Sprachlehrer.\n\n• Stelle Fragen in jeder Sprache\n• Höre deinen Lehrer sprechen\n• Lerne Englisch, Arabisch, Italienisch und mehr\n• Lehrer-Modus: Übungen, Feedback, sokratischer Dialog\n\n👇 Wähle deine Sprache zum Starten:",
    help: "<b>Befehle</b>\n\n/ask <i>[<Text>]</i> — Frage den Lehrer\n/speak <i>[<Text>]</i> — Sprache hören\n/voices — Stimme wählen\n/lang — Sprache ändern\n/teacher <i>[on|off]</i> — Lehrer-Modus\n/reset — Gespräch zurücksetzen\n\n💡 Oder tippe eine Schaltfläche!",
    choose_lang: "🌐 Wähle deine Sprache:",
    lang_set: "✅ Sprache aktualisiert.",
    voice_set: "✅ Stimme gesetzt.",
    tts_fail: "❌ Spracherzeugung fehlgeschlagen.",
    ask_usage: "Verwendung: /ask <deine Frage>",
    speak_usage: "Verwendung: /speak <Text>",
    brain_off: "🧠 KI nicht konfiguriert.",
    reset_done: "🧠 Gesprächsspeicher gelöscht.",
    no_voice_data: "ℹ️ Keine Antwort zum Vorlesen. Frage zuerst mit /ask.",

    teacher_on: "🎓 Lehrer-Modus <b>aktiviert</b>. Ich bringe es dir Schritt für Schritt bei.",
    teacher_off: "🎓 Lehrer-Modus <b>deaktiviert</b>. Zurück zum Assistenten.",
    teacher_status_on: "🎓 Lehrer-Modus: <b>AN</b>",
    teacher_status_off: "🎓 Lehrer-Modus: <b>AUS</b>",
    teacher_usage: "Verwendung: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Sprachlehrer</b>\nWähle:",
    btn_ask: "🧠 Frage stellen",
    btn_speak: "🔊 Sprechen",
    btn_voices: "🎙 Stimmen",
    btn_lang: "🌐 Sprache",
    btn_teacher: "🎓 Lehrer",
    btn_help: "❓ Hilfe",
    btn_reset: "🔄 Reset",

    loading_think: "🧠 Ich denke…",

    voices_header: "🎙 <b>Verfügbare Stimmen</b>\n\nTippe eine Stimme zum Auswählen:",
    listen_hint: "🔊 Tippe eine Stimme zum Anhören",
    unknown_command: "❓ <b>Unbekannter Befehl.</b> Hier ist, was ich kann:",
};

static RU: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Голосовой Учитель</b>\n\nТвой многоязычный ИИ-учитель языков.\n\n• Задавай вопросы на любом языке\n• Слушай учителя\n• Учи английский, арабский, итальянский и ещё\n• Режим учителя: упражнения, обратная связь, сократический диалог\n\n👇 Выбери язык для начала:",
    help: "<b>Команды</b>\n\n/ask <i>[<текст>]</i> — Спроси учителя\n/speak <i>[<текст>]</i> — Услышь речь\n/voices — Выбери голос\n/lang — Смени язык\n/teacher <i>[on|off]</i> — Режим учителя\n/reset — Сбросить диалог\n\n💡 Или нажми кнопку!",
    choose_lang: "🌐 Выберите язык:",
    lang_set: "✅ Язык обновлён.",
    voice_set: "✅ Голос установлен.",
    tts_fail: "❌ Ошибка генерации речи.",
    ask_usage: "Использование: /ask <ваш вопрос>",
    speak_usage: "Использование: /speak <текст>",
    brain_off: "🧠 ИИ не настроен.",
    reset_done: "🧠 Память очищена.",
    no_voice_data: "ℹ️ Нет ответа для озвучки. Спросите сначала.",

    teacher_on: "🎓 Режим учителя <b>включён</b>. Учу пошагово.",
    teacher_off: "🎓 Режим учителя <b>выключен</b>. Назад к ассистенту.",
    teacher_status_on: "🎓 Режим учителя: <b>ВКЛ</b>",
    teacher_status_off: "🎓 Режим учителя: <b>ВЫКЛ</b>",
    teacher_usage: "Использование: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Голосовой Учитель</b>\nВыберите:",
    btn_ask: "🧠 Спросить",
    btn_speak: "🔊 Говорить",
    btn_voices: "🎙 Голоса",
    btn_lang: "🌐 Язык",
    btn_teacher: "🎓 Учитель",
    btn_help: "❓ Помощь",
    btn_reset: "🔄 Сброс",

    loading_think: "🧠 Думаю…",

    voices_header: "🎙 <b>Доступные голоса</b>\n\nНажмите голос для выбора:",
    listen_hint: "🔊 Нажмите голос чтобы услышать",
    unknown_command: "❓ <b>Неизвестная команда.</b> Вот что я могу:",
};

static HI: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — वॉयस टीचर</b>\n\nआपका बहुभाषी AI भाषा शिक्षक।\n\n• किसी भी भाषा में प्रश्न पूछें\n• शिक्षक को बोलते सुनें\n• अंग्रेज़ी, अरबी, इतालवी + और सीखें\n• टीचर मोड: अभ्यास, फ़ीडबैक, सुकराती संवाद\n\n👇 शुरू करने के लिए अपनी भाषा चुनें:",
    help: "<b>कमांड</b>\n\n/ask <i>[<पाठ>]</i> — शिक्षक से पूछें\n/speak <i>[<पाठ>]</i> — बोल सुनें\n/voices — आवाज़ चुनें\n/lang — भाषा बदलें\n/teacher <i>[on|off]</i> — टीचर मोड\n/reset — बातचीत रीसेट\n\n💡 या नीचे बटन दबाएँ!",
    choose_lang: "🌐 अपनी भाषा चुनें:",
    lang_set: "✅ भाषा अपडेट।",
    voice_set: "✅ आवाज़ सेट।",
    tts_fail: "❌ बोलने में विफल। फिर से कोशिश करें।",
    ask_usage: "उपयोग: /ask <आपका प्रश्न>",
    speak_usage: "उपयोग: /speak <बोलने के लिए पाठ>",
    brain_off: "🧠 AI सेट नहीं।",
    reset_done: "🧠 बातचीत साफ़।",
    no_voice_data: "ℹ️ सुनने के लिए कोई जवाब नहीं। पहले /ask पूछें।",

    teacher_on: "🎓 टीचर मोड <b>चालू</b>। मैं आपको कदम-दर-कदम सिखाऊँगा।",
    teacher_off: "🎓 टीचर मोड <b>बंद</b>। वापस असिस्टेंट।",
    teacher_status_on: "🎓 टीचर मोड: <b>चालू</b>",
    teacher_status_off: "🎓 टीचर मोड: <b>बंद</b>",
    teacher_usage: "उपयोग: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — वॉयस टीचर</b>\nचुनें:",
    btn_ask: "🧠 पूछें",
    btn_speak: "🔊 बोलें",
    btn_voices: "🎙 आवाज़ें",
    btn_lang: "🌐 भाषा",
    btn_teacher: "🎓 शिक्षक",
    btn_help: "❓ मदद",
    btn_reset: "🔄 रीसेट",

    loading_think: "🧠 सोच रहा हूँ…",

    voices_header: "🎙 <b>उपलब्ध आवाज़ें</b>\n\nचुनने के लिए आवाज़ टैप करें:",
    listen_hint: "🔊 सुनने के लिए टैप करें",
    unknown_command: "❓ <b>अज्ञात कमांड।</b> मैं यह कर सकता हूँ:",
};

static TR: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Sesli Öğretmen</b>\n\nÇok dilli yapay zeka dil öğretmeni.\n\n• Herhangi bir dille soru sor\n• Öğretmeninin konuştuğunu dinle\n• İngilizce, Arapça, İtalyanca ve daha fazlasını öğren\n• Öğretmen modu: alıştırmalar, geri bildirim, Sokratik diyalog\n\n👇 Başlamak için dilini seç:",
    help: "<b>Komutlar</b>\n\n/ask <i>[<metin>]</i> — Öğretmene sor\n/speak <i>[<metin>]</i> — Konuşmayı dinle\n/voices — Ses seç\n/lang — Dil değiştir\n/teacher <i>[on|off]</i> — Öğretmen modu\n/reset — konuşmayı sıfırla\n\n💡 Ya da aşağıdaki düğmeye dokun!",
    choose_lang: "🌐 Dilinizi seçin:",
    lang_set: "✅ Dil güncellendi.",
    voice_set: "✅ Ses ayarlandı.",
    tts_fail: "❌ Ses oluşturma başarısız.",
    ask_usage: "Kullanım: /ask <sorunuz>",
    speak_usage: "Kullanım: /speak <söyleyeceğiniz metin>",
    brain_off: "🧠 Yapay zeka ayarlanmamış.",
    reset_done: "🧠 Konuşma hafızası temizlendi.",
    no_voice_data: "ℹ️ Dinlenecek yanıt yok. Önce /ask ile sor.",

    teacher_on: "🎓 Öğretmen modu <b>açıldı</b>. Adım adım öğretiyorum.",
    teacher_off: "🎓 Öğretmen modu <b>kapatıldı</b>. Asistana dön.",
    teacher_status_on: "🎓 Öğretmen modu: <b>AÇIK</b>",
    teacher_status_off: "🎓 Öğretmen modu: <b>KAPALI</b>",
    teacher_usage: "Kullanım: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Sesli Öğretmen</b>\nSeçin:",
    btn_ask: "🧠 Sor",
    btn_speak: "🔊 Konuş",
    btn_voices: "🎙 Sesler",
    btn_lang: "🌐 Dil",
    btn_teacher: "🎓 Öğretmen",
    btn_help: "❓ Yardım",
    btn_reset: "🔄 Sıfırla",

    loading_think: "🧠 Düşünüyorum…",

    voices_header: "🎙 <b>Mevcut Sesler</b>\n\nSeçmek için bir sese dokunun:",
    listen_hint: "🔊 Dinlemek için sese dokunun",
    unknown_command: "❓ <b>Bilinmeyen komut.</b> Yapabildiklerim:",
};

static PT: Strings = Strings {
    welcome: "🎓 <b>ANUBIS — Professor de Voz</b>\n\nSeu professor de idiomas IA multilíngue.\n\n• Faça perguntas em qualquer idioma\n• Ouça seu professor falar\n• Aprenda inglês, árabe, italiano e mais\n• Modo professor: exercícios, feedback, diálogo socrático\n\n👇 Escolha seu idioma para começar:",
    help: "<b>Comandos</b>\n\n/ask <i>[<texto>]</i> — Pergunte ao professor\n/speak <i>[<texto>]</i> — Ouça a fala\n/voices — Escolha uma voz\n/lang — Mude o idioma\n/teacher <i>[on|off]</i> — Modo professor\n/reset — Resetar conversa\n\n💡 Ou toque em um botão abaixo!",
    choose_lang: "🌐 Selecione seu idioma:",
    lang_set: "✅ Idioma atualizado.",
    voice_set: "✅ Voz definida.",
    tts_fail: "❌ Falha ao gerar fala.",
    ask_usage: "Uso: /ask <sua pergunta>",
    speak_usage: "Uso: /speak <texto para falar>",
    brain_off: "🧠 IA não configurada.",
    reset_done: "🧠 Memória de conversa apagada.",
    no_voice_data: "ℹ️ Nenhuma resposta para ouvir. Pergunte com /ask.",

    teacher_on: "🎓 Modo professor <b>ativado</b>. Ensino passo a passo.",
    teacher_off: "🎓 Modo professor <b>desativado</b>. Voltando ao assistente.",
    teacher_status_on: "🎓 Modo professor: <b>ON</b>",
    teacher_status_off: "🎓 Modo professor: <b>OFF</b>",
    teacher_usage: "Uso: <i>[/teacher on|off]</i>",

    menu_header: "🎓 <b>ANUBIS — Professor de Voz</b>\nEscolha:",
    btn_ask: "🧠 Perguntar",
    btn_speak: "🔊 Falar",
    btn_voices: "🎙 Vozes",
    btn_lang: "🌐 Idioma",
    btn_teacher: "🎓 Professor",
    btn_help: "❓ Ajuda",
    btn_reset: "🔄 Reset",

    loading_think: "🧠 Pensando…",

    voices_header: "🎙 <b>Vozes Disponíveis</b>\n\nToque para selecionar uma voz:",
    listen_hint: "🔊 Toque para ouvir",
    unknown_command: "❓ <b>Comando desconhecido.</b> Aqui está o que posso fazer:",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LangMeta {
    pub code: &'static str,
    pub label: &'static str,
}

pub const LANGUAGES: &[LangMeta] = &[
    LangMeta {
        code: "en",
        label: "🇬🇧 English",
    },
    LangMeta {
        code: "ar",
        label: "🇸🇦 العربية",
    },
    LangMeta {
        code: "it",
        label: "🇮🇹 Italiano",
    },
    LangMeta {
        code: "fr",
        label: "🇫🇷 Français",
    },
    LangMeta {
        code: "es",
        label: "🇪🇸 Español",
    },
    LangMeta {
        code: "de",
        label: "🇩🇪 Deutsch",
    },
    LangMeta {
        code: "ru",
        label: "🇷🇺 Русский",
    },
    LangMeta {
        code: "hi",
        label: "🇮🇳 हिन्दी",
    },
    LangMeta {
        code: "tr",
        label: "🇹🇷 Türkçe",
    },
    LangMeta {
        code: "pt",
        label: "🇧🇷 Português",
    },
];
