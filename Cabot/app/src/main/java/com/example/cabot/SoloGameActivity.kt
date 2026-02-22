package com.example.cabot

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.example.cabot.game.CabotGame
import com.example.cabot.game.Card
import com.example.cabot.game.PlayerId
import com.example.cabot.game.Rank
import com.example.cabot.game.Suit
import com.google.android.material.button.MaterialButton
import com.example.cabot.game.points


class SoloGameActivity : AppCompatActivity() {

    private lateinit var game: CabotGame
    private val mainHandler = Handler(Looper.getMainLooper())

    // UI - cartes
    private lateinit var tvHumanCard0: TextView
    private lateinit var tvHumanCard1: TextView
    private lateinit var tvHumanCard2: TextView
    private lateinit var tvHumanCard3: TextView

    private lateinit var tvAiCard0: TextView
    private lateinit var tvAiCard1: TextView
    private lateinit var tvAiCard2: TextView
    private lateinit var tvAiCard3: TextView

    // UI - infos
    private lateinit var tvDeckInfo: TextView
    private lateinit var tvDiscardInfo: TextView
    private lateinit var tvHeldCard: TextView
    private lateinit var tvPointsInfo: TextView
    private lateinit var tvStatus: TextView

    // UI - actions
    private lateinit var btnCabot: MaterialButton
    private lateinit var btnDrawDeck: MaterialButton
    private lateinit var btnDrawDiscard: MaterialButton
    private lateinit var btnUsePower: MaterialButton
    private lateinit var btnSkipPower: MaterialButton
    private lateinit var btnEndTurn: MaterialButton

    // Containers
    private lateinit var rowDraw: android.view.View
    private lateinit var rowPowerChoice: android.view.View

    // Carte en main
    private var heldCard: Card? = null

    private enum class LastDrawSource { NONE, DECK, DISCARD }

    private var lastDrawSource: LastDrawSource = LastDrawSource.NONE

    // Phases UI
    private enum class TurnPhase { CHOOSE_DRAW, POWER_DECISION, PLAYING, POWER_INTERACTION }

    private var phase: TurnPhase = TurnPhase.CHOOSE_DRAW

    // Pouvoir en cours
    private enum class PendingPower {
        NONE, LOOK_SELF, LOOK_OPPONENT, SWAP_SELECT_SELF, SWAP_SELECT_OPPONENT
    }

    private var pendingPower: PendingPower = PendingPower.NONE
    private var pendingSwapHumanIndex: Int? = null

    // Sélection (juste visuel)
    private var selectedSlotIndex: Int = 0

    // Score de partie (jusqu'à 7)
    private var humanScore: Int = 0
    private var aiScore: Int = 0
    private val scoreToWin = 7

    // Cabot
    private var cabotAnnounced: Boolean = false
    private var aiTurnsRemainingAfterCabot: Int = 0

    // Révélations temporaires (5 secondes)
    private val humanRevealUntil = LongArray(4) { 0L }
    private val aiRevealUntil = LongArray(4) { 0L }

    private lateinit var btnMultiSwap: MaterialButton
    private lateinit var btnConfirmMulti: MaterialButton

    private var multiMode = false
    private val multiSelected = linkedSetOf<Int>() // indices sélectionnés (0..3)

    private var cabotBy: PlayerId? = null
    private var humanTurnsRemainingAfterAiCabot = 0

    // --- MÉMOIRE DE L'IA (ne "voit" pas les cartes cachées) ---
    private val aiMemory = arrayOfNulls<Card>(4)   // ce que l'IA croit avoir à chaque slot
// null = inconnue



    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_solo_game)

        bindViews()
        setupListeners()

        startNewRound(humanStarts = true) // pour l’instant, on démarre toujours joueur. On ajustera après.
    }

    private fun bindViews() {
        tvHumanCard0 = findViewById(R.id.tvHumanCard0)
        tvHumanCard1 = findViewById(R.id.tvHumanCard1)
        tvHumanCard2 = findViewById(R.id.tvHumanCard2)
        tvHumanCard3 = findViewById(R.id.tvHumanCard3)

        tvAiCard0 = findViewById(R.id.tvAiCard0)
        tvAiCard1 = findViewById(R.id.tvAiCard1)
        tvAiCard2 = findViewById(R.id.tvAiCard2)
        tvAiCard3 = findViewById(R.id.tvAiCard3)

        tvDeckInfo = findViewById(R.id.tvDeckInfo)
        tvDiscardInfo = findViewById(R.id.tvDiscardInfo)
        tvHeldCard = findViewById(R.id.tvHeldCard)
        tvPointsInfo = findViewById(R.id.tvPointsInfo)
        tvStatus = findViewById(R.id.tvStatus)

        btnCabot = findViewById(R.id.btnCabot)
        btnDrawDeck = findViewById(R.id.btnDrawDeck)
        btnDrawDiscard = findViewById(R.id.btnDrawDiscard)
        btnUsePower = findViewById(R.id.btnUsePower)
        btnSkipPower = findViewById(R.id.btnSkipPower)
        btnEndTurn = findViewById(R.id.btnEndTurn)

        rowDraw = findViewById(R.id.rowDraw)
        rowPowerChoice = findViewById(R.id.rowPowerChoice)

        btnMultiSwap = findViewById(R.id.btnMultiSwap)
        btnConfirmMulti = findViewById(R.id.btnConfirmMulti)


        // Clics cartes joueur
        val humanViews = listOf(tvHumanCard0, tvHumanCard1, tvHumanCard2, tvHumanCard3)
        humanViews.forEachIndexed { index, view ->
            view.setOnClickListener {
                val cardInHand = heldCard
                if (cardInHand != null) {

                    // ✅ si multi-mode actif : on sélectionne/désélectionne
                    if (multiMode) {
                        toggleMultiSelection(index)
                        return@setOnClickListener
                    }

                    // sinon comportement normal : remplacer une seule carte
                    val success = game.replaceCardOnBoard(PlayerId.HUMAN, index, cardInHand)
                    if (success) {
                        heldCard = null
                        lastDrawSource = LastDrawSource.NONE
                        setStatus("Carte posée. Vous pouvez finir le tour.")
                        phase = TurnPhase.PLAYING

                        multiMode = false
                        multiSelected.clear()

                        updateActionUi()
                        refreshBoard()
                    } else {
                        setStatus("Impossible de remplacer cette case.")
                    }
                    return@setOnClickListener
                } else {
                    when (pendingPower) {
                        PendingPower.LOOK_SELF -> {
                            revealHuman(index)
                            setStatus("Vous regardez une de vos cartes (5s).")
                            endPowerInteractionToPlaying()
                        }

                        PendingPower.SWAP_SELECT_SELF -> {
                            if (game.state.humanBoard.slots.getOrNull(index) != null) {
                                pendingSwapHumanIndex = index
                                pendingPower = PendingPower.SWAP_SELECT_OPPONENT
                                setStatus("Choisissez une carte adverse à échanger.")
                            } else {
                                setStatus("Choisissez une case avec une carte.")
                            }
                        }

                        else -> {
                            selectedSlotIndex = index
                            updateSelectionUi()
                        }
                    }
                }
            }
        }

        // Clics cartes IA
        val aiViews = listOf(tvAiCard0, tvAiCard1, tvAiCard2, tvAiCard3)
        aiViews.forEachIndexed { index, view ->
            view.setOnClickListener {
                when (pendingPower) {
                    PendingPower.LOOK_OPPONENT -> {
                        revealAi(index)
                        setStatus("Vous regardez une carte adverse (5s).")
                        endPowerInteractionToPlaying()
                    }

                    PendingPower.SWAP_SELECT_OPPONENT -> {
                        val hIdx = pendingSwapHumanIndex
                        val humanSlots = game.state.humanBoard.slots
                        val aiSlots = game.state.aiBoard.slots

                        if (hIdx != null && humanSlots.getOrNull(hIdx) != null && aiSlots.getOrNull(
                                index
                            ) != null
                        ) {
                            val myCard = humanSlots[hIdx]
                            val aiCard = aiSlots[index]
                            humanSlots[hIdx] = aiCard
                            aiSlots[index] = myCard
                            setStatus("Échange effectué.")
                            pendingSwapHumanIndex = null
                            endPowerInteractionToPlaying()
                            refreshBoard()
                        } else {
                            setStatus("Échange impossible.")
                        }
                    }

                    else -> Unit
                }
            }
        }
    }

    private fun setupListeners() {

        // CABOT
        btnCabot.setOnClickListener {
            if (cabotAnnounced) return@setOnClickListener
            if (pendingPower != PendingPower.NONE || phase == TurnPhase.POWER_INTERACTION) {
                setStatus("Terminez d'abord le pouvoir en cours.")
                return@setOnClickListener
            }

            val isStartOfTurn = (phase == TurnPhase.CHOOSE_DRAW)

            AlertDialog.Builder(this)
                .setTitle("Cabot")
                .setMessage(
                    if (isStartOfTurn)
                        "Dire Cabot maintenant annule votre tour. L'IA jouera 1 coup, puis on comptera les points."
                    else
                        "Dire Cabot maintenant donne 2 coups à l'IA, puis on comptera les points."
                )
                .setPositiveButton("Dire Cabot") { _, _ ->
                    cabotAnnounced = true
                    aiTurnsRemainingAfterCabot = if (isStartOfTurn) 1 else 2

                    // Si c'est le début de tour : tour annulé => IA joue tout de suite
                    // Si c'est la fin : on finira ton tour normalement avec Fin de tour (mais on peut aussi forcer le passage)
                    setStatus("Cabot annoncé. Fin de manche imminente.")
                    cabotBy = PlayerId.HUMAN
                    // On force le passage à l'IA immédiatement
                    forcePassToAiAndResolveCabot()
                }
                .setNegativeButton("Annuler", null)
                .show()
        }

        // Début de tour : pioche pot
        btnDrawDeck.setOnClickListener {
            if (phase != TurnPhase.CHOOSE_DRAW) return@setOnClickListener

            val card = game.drawFromDeck()
            if (card == null) {
                setStatus("Pioche vide.")
                return@setOnClickListener
            }

            heldCard = card
            lastDrawSource = LastDrawSource.DECK

            if (isPowerCard(card)) {
                setStatus("Carte pouvoir piochée. Utiliser le pouvoir ?")
                phase = TurnPhase.POWER_DECISION
            } else {
                setStatus("Posez la carte, ou finissez le tour pour la défausser.")
                phase = TurnPhase.PLAYING
            }

            updateActionUi()
            refreshBoard()
        }

        // Début de tour : pioche poubelle
        btnDrawDiscard.setOnClickListener {
            if (phase != TurnPhase.CHOOSE_DRAW) return@setOnClickListener

            val card = game.drawFromDiscard()
            if (card == null) {
                setStatus("Poubelle vide.")
                return@setOnClickListener
            }

            heldCard = card
            lastDrawSource = LastDrawSource.DISCARD

            setStatus("Posez la carte, ou finissez le tour pour la défausser.")
            phase = TurnPhase.PLAYING
            updateActionUi()
            refreshBoard()
        }

        // Utiliser pouvoir (carte pouvoir piochée du pot)
        btnUsePower.setOnClickListener {
            if (phase != TurnPhase.POWER_DECISION) return@setOnClickListener
            val card = heldCard ?: return@setOnClickListener

            // Défausse + activation du pouvoir
            game.discard(card)
            heldCard = null
            lastDrawSource = LastDrawSource.NONE

            startPendingPower(card)
            phase = TurnPhase.POWER_INTERACTION
            updateActionUi()
            refreshBoard()
        }

        // Ne pas utiliser pouvoir
        btnSkipPower.setOnClickListener {
            if (phase != TurnPhase.POWER_DECISION) return@setOnClickListener
            val card = heldCard ?: return@setOnClickListener

            setStatus("Pouvoir ignoré. Posez la carte, ou finissez le tour pour la défausser.")
            phase = TurnPhase.PLAYING
            updateActionUi()
            refreshBoard()
        }

        // Fin de tour
        // Fin de tour
        btnEndTurn.setOnClickListener {
            if (pendingPower != PendingPower.NONE) {
                setStatus("Terminez d'abord le pouvoir.")
                return@setOnClickListener
            }

            // si carte en main => défausse
            heldCard?.let {
                game.discard(it)
                setStatus("Vous défaussez votre carte.")
                heldCard = null
                lastDrawSource = LastDrawSource.NONE
            }

            // Passage normal
            game.endTurn()

            // --- CAS 1 : Cabot annoncé par le JOUEUR ---
            if (cabotAnnounced && cabotBy == PlayerId.HUMAN) {
                resolveCabotSequence()
                return@setOnClickListener
            }

            // --- CAS 2 : Cabot annoncé par l'IA ---
            if (cabotAnnounced && cabotBy == PlayerId.AI) {
                humanTurnsRemainingAfterAiCabot--

                if (humanTurnsRemainingAfterAiCabot <= 0) {
                    endRoundAndShowResult()
                    return@setOnClickListener
                }

                // Redonne la main au joueur (annule le switch vers IA)
                game.endTurn()
                setStatus("Il vous reste $humanTurnsRemainingAfterAiCabot coup(s) (Cabot IA).")
                phase = TurnPhase.CHOOSE_DRAW
                updateActionUi()
                refreshBoard()
                return@setOnClickListener
            }

            // --- TOUR IA standard ---
            if (game.state.currentPlayer == PlayerId.AI) {
                aiTakeTurn()
            } else {
                setStatus("À vous : choisissez où piocher.")
            }

            phase = TurnPhase.CHOOSE_DRAW
            updateActionUi()
            refreshBoard()
        }

// ✅ EN DEHORS du btnEndTurn :
        btnMultiSwap.setOnClickListener {
            if (heldCard == null) return@setOnClickListener
            multiMode = !multiMode
            multiSelected.clear()
            setStatus(
                if (multiMode) "Mode échange multiple : sélectionnez vos cartes, puis validez."
                else "Mode échange multiple désactivé."
            )
            updateActionUi()
            updateSelectionUi()
        }

        btnConfirmMulti.setOnClickListener {
            if (!multiMode) return@setOnClickListener
            applyMultiSwapOrPenalty()
        }
    }
    // --- CABOT FLOW ---

    private fun forcePassToAiAndResolveCabot() {
        // On annule l'éventuelle carte en main (logique simple : on la défausse)
        heldCard?.let { game.discard(it) }
        heldCard = null
        lastDrawSource = LastDrawSource.NONE

        // On force la main à l'IA
        if (game.state.currentPlayer == PlayerId.HUMAN) game.endTurn()

        resolveCabotSequence()
    }

    private fun resolveCabotSequence() {
        // IA joue aiTurnsRemainingAfterCabot tours
        while (aiTurnsRemainingAfterCabot > 0) {
            setStatus("IA joue (${aiTurnsRemainingAfterCabot} coup(s) restant(s))...")
            game.playBasicAiTurn()
            aiTurnsRemainingAfterCabot--
            // alterne joueur
            game.endTurn()
            // repasse la main à l'IA si besoin
            if (aiTurnsRemainingAfterCabot > 0 && game.state.currentPlayer == PlayerId.HUMAN) {
                game.endTurn()
            }
        }

        endRoundAndShowResult()
    }

    private fun aiAnnounceCabot(startOfAiTurn: Boolean) {
        cabotAnnounced = true
        cabotBy = PlayerId.AI
        humanTurnsRemainingAfterAiCabot = if (startOfAiTurn) 1 else 2

        AlertDialog.Builder(this)
            .setTitle("Cabot")
            .setMessage("L'IA dit Cabot.\nIl vous reste $humanTurnsRemainingAfterAiCabot coup(s) à jouer.")
            .setCancelable(false)
            .setPositiveButton("OK", null)
            .show()

        setStatus("L'IA a dit Cabot. Jouez vos $humanTurnsRemainingAfterAiCabot coup(s).")
        phase = TurnPhase.CHOOSE_DRAW
        updateActionUi()
        refreshBoard()
    }


    private fun endRoundAndShowResult() {
        val humanPoints = game.getPlayerPoints(PlayerId.HUMAN)
        val aiPoints = game.getPlayerPoints(PlayerId.AI)

        val humanWins = humanPoints < aiPoints
        if (humanWins) humanScore++ else aiScore++

        val winnerText = if (humanWins) "Vous gagnez la manche !" else "L'IA gagne la manche."
        val scoreText = "Score : Vous $humanScore - $aiScore IA"

        AlertDialog.Builder(this)
            .setTitle("Fin de manche")
            .setMessage("$winnerText\n\nVos points : $humanPoints\nIA : $aiPoints\n\n$scoreText")
            .setCancelable(false)
            .setPositiveButton(if (humanScore >= scoreToWin || aiScore >= scoreToWin) "OK" else "Manche suivante") { _, _ ->
                if (humanScore >= scoreToWin || aiScore >= scoreToWin) {
                    showEndOfGameDialog()
                } else {
                    // Le perdant commence la manche suivante (règle)
                    val humanStartsNext = !humanWins // si l'humain a perdu, il commence
                    startNewRound(humanStarts = humanStartsNext)
                }
            }
            .show()
    }

    private fun showEndOfGameDialog() {
        val title = if (humanScore >= scoreToWin) "Victoire !" else "Défaite"
        val msg = "Score final : Vous $humanScore - $aiScore IA"
        AlertDialog.Builder(this)
            .setTitle(title)
            .setMessage(msg)
            .setCancelable(false)
            .setPositiveButton("Rejouer") { _, _ ->
                humanScore = 0
                aiScore = 0
                startNewRound(humanStarts = true)
            }
            .setNegativeButton("Quitter") { _, _ -> finish() }
            .show()
    }

    private fun startNewRound(humanStarts: Boolean) {
        // ✅ Nouvelle manche = nouveau jeu => deck remélangé + boards redistribués
        game = CabotGame(if (humanStarts) PlayerId.HUMAN else PlayerId.AI)

        // --- INIT MÉMOIRE IA : elle connaît seulement ses 2 cartes du bas (2 et 3) ---
        for (i in 0..3) aiMemory[i] = null
        aiMemory[2] = game.state.aiBoard.slots[2]
        aiMemory[3] = game.state.aiBoard.slots[3]

        // ✅ sécurité : poubelle vidée (même si CabotGame change plus tard)
        game.state.discard.clear()

        // reset UI / états
        cabotAnnounced = false
        cabotBy = null
        aiTurnsRemainingAfterCabot = 0
        humanTurnsRemainingAfterAiCabot = 0

        heldCard = null
        lastDrawSource = LastDrawSource.NONE
        pendingPower = PendingPower.NONE
        pendingSwapHumanIndex = null

        multiMode = false
        multiSelected.clear()

        // reset reveal timers
        for (i in 0..3) {
            humanRevealUntil[i] = 0L
            aiRevealUntil[i] = 0L
        }

        // Début de manche : montrer tes 2 cartes du bas pendant 5 secondes
        revealHuman(2)
        revealHuman(3)
        setStatus("Début de manche : mémorisez vos 2 cartes du bas (5s).")

        phase = TurnPhase.CHOOSE_DRAW
        updateActionUi()
        refreshBoard()

        // ✅ Si IA commence : on doit utiliser aiTakeTurn() (pas playBasicAiTurn())
        if (game.state.currentPlayer == PlayerId.AI) {
            setStatus("L'IA commence la manche...")
            mainHandler.postDelayed({
                aiTakeTurn()
                // aiTakeTurn() finit déjà en HUMAN + status
                phase = TurnPhase.CHOOSE_DRAW
                updateActionUi()
                refreshBoard()
            }, 600L)
        } else {
            setStatus("À vous : choisissez où piocher.")
        }
    }


    // --- POUVOIRS ---

    private fun startPendingPower(card: Card) {
        pendingSwapHumanIndex = null
        pendingPower = when (card.rank) {
            Rank.SEVEN, Rank.EIGHT -> {
                setStatus("Pouvoir : touchez une de VOS cartes pour la voir 5s.")
                PendingPower.LOOK_SELF
            }

            Rank.NINE, Rank.TEN -> {
                setStatus("Pouvoir : touchez une carte IA pour la voir 5s.")
                PendingPower.LOOK_OPPONENT
            }

            Rank.JACK, Rank.QUEEN -> {
                setStatus("Pouvoir : touchez une de vos cartes, puis une carte IA pour échanger.")
                PendingPower.SWAP_SELECT_SELF
            }

            else -> PendingPower.NONE
        }
    }

    private fun endPowerInteractionToPlaying() {
        pendingPower = PendingPower.NONE
        pendingSwapHumanIndex = null
        phase = TurnPhase.PLAYING
        updateActionUi()
        refreshBoard()
    }

    // --- RÉVÉLATION 5 SECONDES ---

    private fun revealHuman(index: Int) {
        val now = System.currentTimeMillis()
        humanRevealUntil[index] = now + 5000L
        refreshBoard()
        mainHandler.postDelayed({ refreshBoard() }, 5100L)
    }

    private fun revealAi(index: Int) {
        val now = System.currentTimeMillis()
        aiRevealUntil[index] = now + 5000L
        refreshBoard()
        mainHandler.postDelayed({ refreshBoard() }, 5100L)
    }

    private fun isHumanRevealed(index: Int): Boolean =
        System.currentTimeMillis() < humanRevealUntil[index]

    private fun isAiRevealed(index: Int): Boolean =
        System.currentTimeMillis() < aiRevealUntil[index]

    // --- UI ---

    private fun updateActionUi() {
        // CABOT visible uniquement pendant ton tour
        btnCabot.visibility = android.view.View.VISIBLE

        // Par défaut on cache les boutons multi
        btnMultiSwap.visibility = android.view.View.GONE
        btnConfirmMulti.visibility = android.view.View.GONE

        when (phase) {
            TurnPhase.CHOOSE_DRAW -> {
                rowDraw.visibility = android.view.View.VISIBLE
                rowPowerChoice.visibility = android.view.View.GONE
                btnEndTurn.visibility = android.view.View.GONE
            }

            TurnPhase.POWER_DECISION -> {
                rowDraw.visibility = android.view.View.GONE
                rowPowerChoice.visibility = android.view.View.VISIBLE
                btnEndTurn.visibility = android.view.View.GONE
            }

            TurnPhase.POWER_INTERACTION -> {
                rowDraw.visibility = android.view.View.GONE
                rowPowerChoice.visibility = android.view.View.GONE
                btnEndTurn.visibility = android.view.View.GONE
            }

            TurnPhase.PLAYING -> {
                rowDraw.visibility = android.view.View.GONE
                rowPowerChoice.visibility = android.view.View.GONE
                btnEndTurn.visibility = android.view.View.VISIBLE

                // ✅ IMPORTANT : ici on affiche l’échange multiple si :
                // - tu as une carte en main
                // - aucun pouvoir en cours
                val canMulti = (heldCard != null) && (pendingPower == PendingPower.NONE)

                btnMultiSwap.visibility =
                    if (canMulti) android.view.View.VISIBLE else android.view.View.GONE
                btnConfirmMulti.visibility =
                    if (canMulti && multiMode) android.view.View.VISIBLE else android.view.View.GONE
            }
        }
    }

    private fun refreshBoard() {
        val state = game.state
        val humanSlots = state.humanBoard.slots
        val aiSlots = state.aiBoard.slots

        // ⚠️ IMPORTANT : par défaut on masque TOUT (même si “connu”), sauf pendant une fenêtre de 5s
        tvHumanCard0.text = displayHumanSlot(0, humanSlots[0])
        tvHumanCard1.text = displayHumanSlot(1, humanSlots[1])
        tvHumanCard2.text = displayHumanSlot(2, humanSlots[2])
        tvHumanCard3.text = displayHumanSlot(3, humanSlots[3])

        tvAiCard0.text = displayAiSlot(0, aiSlots[0])
        tvAiCard1.text = displayAiSlot(1, aiSlots[1])
        tvAiCard2.text = displayAiSlot(2, aiSlots[2])
        tvAiCard3.text = displayAiSlot(3, aiSlots[3])

        tvDeckInfo.text = "Pioche : ${state.deck.size} cartes"
        tvDiscardInfo.text = if (state.discard.isEmpty()) "Poubelle : (vide)"
        else "Poubelle : ${cardToString(state.discard.last())}"

        tvHeldCard.text = "Carte en main : ${cardToString(heldCard)}"

        val humanPoints = game.getPlayerPoints(PlayerId.HUMAN)
        val aiPoints = game.getPlayerPoints(PlayerId.AI)
        tvPointsInfo.text =
            "Manche : Vous $humanPoints | IA ?    •    Partie : $humanScore - $aiScore"

        updateSelectionUi()
    }

    private fun displayHumanSlot(index: Int, card: Card?): String {
        if (card == null) return "—"
        return if (isHumanRevealed(index)) cardToString(card) else "??"
    }

    private fun displayAiSlot(index: Int, card: Card?): String {
        if (card == null) return "—"
        return if (isAiRevealed(index)) cardToString(card) else "??"
    }

    private fun updateSelectionUi() {
        val views = listOf(tvHumanCard0, tvHumanCard1, tvHumanCard2, tvHumanCard3)

        views.forEachIndexed { index, view ->
            val isSelected = if (multiMode) {
                multiSelected.contains(index)
            } else {
                index == selectedSlotIndex
            }

            val bg = if (isSelected) {
                R.drawable.bg_board_card_selected
            } else {
                R.drawable.bg_board_card
            }

            view.setBackgroundResource(bg)
        }
    }

    private fun setStatus(msg: String) {
        tvStatus.text = msg
    }

    private fun isPowerCard(card: Card): Boolean {
        return when (card.rank) {
            Rank.SEVEN, Rank.EIGHT,
            Rank.NINE, Rank.TEN,
            Rank.JACK, Rank.QUEEN -> true

            else -> false
        }
    }

    private fun cardToString(card: Card?): String {
        if (card == null) return "—"
        val rankStr = when (card.rank) {
            Rank.TWO -> "2"
            Rank.THREE -> "3"
            Rank.FOUR -> "4"
            Rank.FIVE -> "5"
            Rank.SIX -> "6"
            Rank.SEVEN -> "7"
            Rank.EIGHT -> "8"
            Rank.NINE -> "9"
            Rank.TEN -> "10"
            Rank.JACK -> "J"
            Rank.QUEEN -> "Q"
            Rank.KING -> "K"
            Rank.ACE -> "A"
            Rank.JOKER -> "Jkr"
        }
        val suitStr = when (card.suit) {
            Suit.HEARTS -> "♥"
            Suit.DIAMONDS -> "♦"
            Suit.CLUBS -> "♣"
            Suit.SPADES -> "♠"
            Suit.JOKER -> ""
        }
        return if (card.rank == Rank.JOKER) "Joker" else "$rankStr$suitStr"
    }

    private fun toggleMultiSelection(index: Int) {
        if (multiSelected.contains(index)) multiSelected.remove(index) else multiSelected.add(
            index
        )
        setStatus("Sélection : ${multiSelected.map { it + 1 }.sorted().joinToString(", ")}")
        updateSelectionUi()
        updateActionUi()
    }

    private fun applyMultiSwapOrPenalty() {
        val cardInHand = heldCard ?: return
        if (multiSelected.isEmpty()) {
            setStatus("Sélectionnez au moins 1 carte.")
            return
        }

        val slots = game.state.humanBoard.slots
        val selectedCards = multiSelected.mapNotNull { slots[it] }

        // si une case vide a été sélectionnée, c'est automatiquement invalide
        if (selectedCards.size != multiSelected.size) {
            applyPenalty(cardInHand)
            return
        }

        val allSameRank = selectedCards.all { it.rank == selectedCards[0].rank }

        if (!allSameRank) {
            applyPenalty(cardInHand)
            return
        }

        // ✅ OK : on défausse toutes les cartes sélectionnées
        selectedCards.forEach { game.discard(it) }

        // on place la carte en main sur la première case sélectionnée
        val targetIndex = multiSelected.first()
        slots[targetIndex] = cardInHand

        // les autres cases sélectionnées deviennent vides
        multiSelected.drop(1).forEach { idx -> slots[idx] = null }

        heldCard = null
        lastDrawSource = LastDrawSource.NONE

        setStatus("Échange multiple réussi. Fin de tour disponible.")
        multiMode = false
        multiSelected.clear()
        phase = TurnPhase.PLAYING
        updateActionUi()
        refreshBoard()
    }

    private fun applyPenalty(cardInHand: Card) {
        // pool = toutes tes cartes + carte en main
        val slots = game.state.humanBoard.slots
        val pool = mutableListOf<Card>()
        slots.forEach { if (it != null) pool.add(it) }
        pool.add(cardInHand)

        // carte punie au hasard
        val punished = pool.random()
        game.discard(punished)

        // retire punished du pool (1 occurrence)
        pool.remove(punished)

        // mélange le reste et redistribue
        pool.shuffle()
        for (i in 0..3) slots[i] = pool.getOrNull(i)

        // carte en main consommée (quoi qu’il arrive)
        heldCard = null
        lastDrawSource = LastDrawSource.NONE

        multiMode = false
        multiSelected.clear()

        // fin de tour forcée : on affiche, puis on passe la main
        androidx.appcompat.app.AlertDialog.Builder(this)
            .setTitle("Erreur d’échange")
            .setMessage("Les cartes sélectionnées n’ont pas la même valeur.\nPénalité : une carte est défaussée, vos cartes sont mélangées.\nFin de tour forcée.")
            .setCancelable(false)
            .setPositiveButton("OK") { _, _ ->
                forceEndTurnAfterPenalty()
            }
            .show()
    }

    private fun forceEndTurnAfterPenalty() {
        // On passe la main immédiatement (comme si tu avais cliqué fin de tour)
        if (pendingPower != PendingPower.NONE) pendingPower = PendingPower.NONE
        game.endTurn()

        // IA joue
        if (game.state.currentPlayer == PlayerId.AI) {
            setStatus("Tour de l'IA...")
            game.playBasicAiTurn()
            game.endTurn()
        }

        setStatus("À vous : choisissez où piocher.")
        phase = TurnPhase.CHOOSE_DRAW
        updateActionUi()
        refreshBoard()
    }

    private fun showAiExplanation(lines: List<String>) {
        if (lines.isEmpty()) return

        AlertDialog.Builder(this)
            .setTitle("Tour de l'IA")
            .setMessage(lines.joinToString("\n"))
            .setPositiveButton("OK", null)
            .show()
    }

    private fun aiTakeTurn() {
        val log = StringBuilder()

        fun slotName(i: Int) = (i + 1).toString()
        fun unknownPoints(i: Int): Int = aiMemory[i]?.points() ?: 99

        setStatus("Tour de l'IA...")

        val aiPointsStart = game.getPlayerPoints(PlayerId.AI)

        // Cabot IA au début si déjà < 6
        if (!cabotAnnounced && aiPointsStart < 6) {
            if (game.state.currentPlayer == PlayerId.AI) game.endTurn()
            aiAnnounceCabot(startOfAiTurn = true)
            return
        }

        // 1) Choix de pioche
        val discardTop = game.state.discard.lastOrNull()
        val drewFromDiscard = (discardTop != null && discardTop.points() <= 4)

        val drawn: Card? = if (drewFromDiscard) {
            game.drawFromDiscard()
        } else {
            game.drawFromDeck() ?: game.drawFromDiscard()
        }

        if (drawn == null) {
            game.endTurn()
            setStatus("À vous : choisissez où piocher.")
            return
        }

        log.append(
            if (drewFromDiscard) "IA pioche dans la poubelle. "
            else "IA pioche dans la pioche. "
        )

        val aiSlots = game.state.aiBoard.slots
        val nonEmpty = aiSlots.indices.filter { aiSlots[it] != null }

        if (nonEmpty.isEmpty()) {
            game.discard(drawn)
            log.append("Elle défausse la carte (aucune carte à remplacer).")
            setStatus(log.toString())
            game.endTurn()
            return
        }

        // 2) Tentative de multi-discard seulement sur cartes connues (aiMemory)
        val knownIndices = nonEmpty.filter { aiMemory[it] != null }
        val groups = knownIndices
            .groupBy { aiMemory[it]!!.rank }
            .filter { it.value.size >= 2 }

        val bestGroup = groups.maxByOrNull { (_, idxs) ->
            idxs.sumOf { aiMemory[it]!!.points() }
        }

        if (bestGroup != null) {
            val idxs = bestGroup.value.sorted()
            val keptIndex = idxs.first()

            log.append("Elle défausse plusieurs cartes identiques (cases ${idxs.joinToString { slotName(it) }}). ")
            log.append("Puis elle pose la carte sur la case ${slotName(keptIndex)}.")

            // Défausse toutes les cartes du groupe
            idxs.forEach { idx ->
                aiSlots[idx]?.let { game.discard(it) }
                aiSlots[idx] = null
                aiMemory[idx] = null
            }

            // Pose la carte piochée
            aiSlots[keptIndex] = drawn
            aiMemory[keptIndex] = drawn

            setStatus(log.toString())
            game.endTurn()

            // Cabot IA à la fin si < 6
            val aiPointsEnd = game.getPlayerPoints(PlayerId.AI)
            if (!cabotAnnounced && aiPointsEnd < 6) {
                aiAnnounceCabot(startOfAiTurn = false)
                return
            }

            return
        }

        // 3) Sinon : remplace la "pire" case selon sa mémoire (inconnue = très mauvaise)
        val worstIdx = nonEmpty.maxBy { unknownPoints(it) }

        // Décision censurée : on ne révèle pas la valeur, juste l'action
        val shouldReplace = (drawn.points() <= 4) || (drawn.points() < (aiMemory[worstIdx]?.points() ?: 11))

        if (shouldReplace) {
            game.replaceCardOnBoard(PlayerId.AI, worstIdx, drawn)
            aiMemory[worstIdx] = drawn
            log.append("Elle remplace la case ${slotName(worstIdx)}.")
        } else {
            game.discard(drawn)
            log.append("Elle défausse la carte.")
        }

        setStatus(log.toString())

        game.endTurn()

        // Cabot IA à la fin si < 6
        val aiPointsEnd = game.getPlayerPoints(PlayerId.AI)
        if (!cabotAnnounced && aiPointsEnd < 6) {
            aiAnnounceCabot(startOfAiTurn = false)
            return
        }
    }
}
