package com.example.cabot

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        // Version affichée en dur pour l'instant
        val versionView = findViewById<TextView>(R.id.tvVersion)
        versionView.text = "v1.0 (1)"

        // Bouton "Jouer"
        findViewById<View>(R.id.btnPlay).setOnClickListener {
            // Avant : startActivity(Intent(this, GameActivity::class.java))
            startActivity(Intent(this, SoloGameActivity::class.java))
        }

        // Bouton "Règles"
        findViewById<View>(R.id.btnRules).setOnClickListener {
            startActivity(Intent(this, RulesActivity::class.java))
        }

        // Bouton "Paramètres"
        findViewById<View>(R.id.btnSettings).setOnClickListener {
            startActivity(Intent(this, SettingsActivity::class.java))
        }
    }
}
