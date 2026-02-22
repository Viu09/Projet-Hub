package com.example.cabot

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.view.animation.Animation
import android.view.animation.AnimationUtils
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.button.MaterialButton

class RulesActivity : AppCompatActivity() {

    data class RuleStep(
        val title: String,
        val body: String
    )

    private lateinit var steps: List<RuleStep>
    private var currentIndex = 0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_rules)

        steps = createSteps()

        val contentContainer = findViewById<View>(R.id.rulesContentContainer)
        val tvStepIndicator = findViewById<TextView>(R.id.tvStepIndicator)
        val tvStepTitle = findViewById<TextView>(R.id.tvStepTitle)
        val tvStepBody = findViewById<TextView>(R.id.tvStepBody)
        val btnPrev = findViewById<MaterialButton>(R.id.btnPrevStep)
        val btnNext = findViewById<MaterialButton>(R.id.btnNextStep)
        val btnFinish = findViewById<MaterialButton>(R.id.btnFinishRules)

        fun applyTexts() {
            val step = steps[currentIndex]
            tvStepTitle.text = step.title
            tvStepBody.text = step.body
            tvStepIndicator.text = "${currentIndex + 1} / ${steps.size}"

            // états des boutons
            btnPrev.isEnabled = currentIndex > 0
            btnPrev.alpha = if (btnPrev.isEnabled) 1f else 0.4f

            if (currentIndex == steps.lastIndex) {
                btnNext.visibility = View.GONE
                btnFinish.visibility = View.VISIBLE
            } else {
                btnNext.visibility = View.VISIBLE
                btnNext.isEnabled = true
                btnNext.alpha = 1f
                btnFinish.visibility = View.GONE
            }
        }

        fun animateToStep(direction: Int) {
            // direction > 0 : vers la droite (step suivant)
            // direction < 0 : vers la gauche (step précédent)
            if (direction == 0) {
                applyTexts()
                return
            }

            val outAnimRes = if (direction > 0) {
                R.anim.slide_out_left
            } else {
                R.anim.slide_out_right
            }
            val inAnimRes = if (direction > 0) {
                R.anim.slide_in_right
            } else {
                R.anim.slide_in_left
            }

            val outAnim = AnimationUtils.loadAnimation(this, outAnimRes)
            val inAnim = AnimationUtils.loadAnimation(this, inAnimRes)

            outAnim.setAnimationListener(object : Animation.AnimationListener {
                override fun onAnimationStart(animation: Animation?) {}

                override fun onAnimationEnd(animation: Animation?) {
                    applyTexts()
                    contentContainer.startAnimation(inAnim)
                }

                override fun onAnimationRepeat(animation: Animation?) {}
            })

            contentContainer.startAnimation(outAnim)
        }

        btnPrev.setOnClickListener {
            if (currentIndex > 0) {
                currentIndex--
                animateToStep(-1)
            }
        }

        btnNext.setOnClickListener {
            if (currentIndex < steps.lastIndex) {
                currentIndex++
                animateToStep(1)
            }
        }

        btnFinish.setOnClickListener {
            startActivity(Intent(this, MainActivity::class.java))
            finish()
        }

        // première étape : sans animation
        animateToStep(0)
    }

    private fun createSteps(): List<RuleStep> {
        return listOf(
            RuleStep(
                title = "Composition du jeu",
                body = "Cabot se joue avec un paquet complet de 54 cartes : les cartes de 2 à As dans les quatre couleurs, plus les deux jokers."
            ),
            RuleStep(
                title = "Distribution initiale",
                body = "Un joueur est désigné au hasard comme donneur. Il distribue les cartes selon les règles de mise en place. " +
                        "C’est l’autre joueur (celui qui ne distribue pas) qui commencera la première manche."
            ),
            RuleStep(
                title = "Terrain de jeu",
                body = "Chaque joueur possède devant lui un carré de 4 cartes, posées face cachée, en deux rangées de deux. " +
                        "Entre les deux joueurs se trouvent :\n- la pioche (le pot principal),\n- un emplacement pour la défausse, appelée « poubelle »."
            ),
            RuleStep(
                title = "Valeur et pouvoirs des cartes",
                body = "• Cartes 2 à 10 : leur valeur en points correspond à leur chiffre.\n" +
                        "• Valet, Dame, Roi : chacun vaut 10 points.\n" +
                        "• As : 1 point.\n" +
                        "• Joker : 0 point.\n\n" +
                        "Pouvoirs spéciaux :\n" +
                        "• 7 et 8 : regarder une de SES propres cartes.\n" +
                        "• 9 et 10 : regarder une carte de l’ADVERSAIRE.\n" +
                        "• Valet et Dame : échanger une de ses cartes avec une carte de l’adversaire."
            ),
            RuleStep(
                title = "Objectif et déclaration « Cabot »",
                body = "Le but est d’avoir moins de points que l’adversaire dans son carré de 4 cartes.\n\n" +
                        "Quand un joueur pense avoir moins de points que l’autre, il peut déclarer « Cabot » :\n" +
                        "• S’il le dit AVANT de jouer son tour : son tour est annulé et l’adversaire joue encore UN coup.\n" +
                        "• S’il le dit APRÈS avoir joué son tour : l’adversaire joue DEUX coups supplémentaires.\n\n" +
                        "À la fin de ces coups, on révèle les cartes et on compte les points."
            ),
            RuleStep(
                title = "Fin de manche et points de manche",
                body = "Après une déclaration de « Cabot » et les derniers tours de l’adversaire :\n\n" +
                        "• Si le joueur qui a dit « Cabot » a effectivement moins de points, il marque 1 point de manche.\n" +
                        "• Si les points sont égaux ou s’il a plus de points, c’est l’adversaire qui marque 1 point."
            ),
            RuleStep(
                title = "Fin de partie et premier joueur",
                body = "Une partie se joue en plusieurs manches :\n\n" +
                        "• Le premier joueur à atteindre 7 points de manche gagne la partie.\n" +
                        "• Le joueur qui vient de PERDRE la manche commence la manche suivante."
            ),
            RuleStep(
                title = "Apprendre ses cartes",
                body = "Pour bien jouer, il faut connaître ses cartes. Trois moyens :\n\n" +
                        "1) Au début de la manche, chaque joueur peut regarder les deux cartes de la première rangée pendant quelques secondes, puis les reposer face cachée.\n" +
                        "2) Jouer un 7 ou un 8 permet de regarder une de ses cartes face cachée.\n" +
                        "3) À son tour, un joueur pioche soit la dernière carte de la poubelle (visible), soit la première carte du pot (visible uniquement pour lui). " +
                        "S’il garde cette carte, il la place face cachée à la place d’une de ses cartes et défausse l’ancienne dans la poubelle."
            ),
            RuleStep(
                title = "Se débarrasser des cartes",
                body = "En construisant son jeu, un joueur peut réduire ses points en se débarrassant de certaines cartes :\n\n" +
                        "• Une carte piochée peut remplacer une de ses cartes, l’ancienne allant dans la poubelle.\n" +
                        "• Si le joueur forme une paire, un brelan ou un carré (2, 3 ou 4 cartes de même valeur), il peut défausser tout le groupe en un seul coup.\n\n" +
                        "Exemple : avec une paire de 6 et un 2 pioché, il peut défausser les deux 6 et ne garder que le 2. " +
                        "Son carré de départ peut ainsi se réduire jusqu’à un minimum de 1 carte."
            ),
            RuleStep(
                title = "Erreur sur un groupe de cartes",
                body = "Si un joueur se trompe en défaussant une paire, un brelan ou un carré (par exemple en croyant former une paire alors que les cartes ne correspondent pas) :\n\n" +
                        "• Toutes les cartes de son jeu sont récupérées et mélangées.\n" +
                        "• L’adversaire choisit une carte au hasard : elle est défaussée face visible dans la poubelle.\n" +
                        "• Le reste des cartes est reposé face cachée devant le joueur, sans qu’il puisse les regarder de nouveau " +
                        "(sauf en utilisant à nouveau des cartes 7 ou 8, ou des échanges)."
            )
        )
    }
}
