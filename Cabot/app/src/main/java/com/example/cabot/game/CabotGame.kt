package com.example.cabot.game

import kotlin.random.Random

// Couleurs des cartes (joker à part)
enum class Suit {
    HEARTS, DIAMONDS, CLUBS, SPADES, JOKER
}

// Valeurs des cartes
enum class Rank {
    TWO, THREE, FOUR, FIVE, SIX, SEVEN, EIGHT, NINE, TEN,
    JACK, QUEEN, KING, ACE,
    JOKER
}

// Une carte unique
data class Card(
    val suit: Suit,
    val rank: Rank,
    val id: Int   // id unique pour différencier les cartes
)

// Valeur en points d'une carte
fun Card.points(): Int = when (rank) {
    Rank.JOKER -> 0
    Rank.ACE -> 1
    Rank.TWO -> 2
    Rank.THREE -> 3
    Rank.FOUR -> 4
    Rank.FIVE -> 5
    Rank.SIX -> 6
    Rank.SEVEN -> 7
    Rank.EIGHT -> 8
    Rank.NINE -> 9
    Rank.TEN, Rank.JACK, Rank.QUEEN, Rank.KING -> 10
}

// Pour distinguer humain et IA
enum class PlayerId { HUMAN, AI }

private fun <T> MutableList<T>.popLast(): T = this.removeAt(this.size - 1)

// L'état du "carré" de 4 cartes d'un joueur
data class PlayerBoard(
    // 4 emplacements (null si la carte a été défaussée plus tard)
    val slots: MutableList<Card?> = MutableList(4) { null }
) {
    fun totalPoints(): Int =
        slots.filterNotNull().sumOf { it.points() }
}

// État global d'une manche
data class CabotGameState(
    val deck: MutableList<Card>,         // pioche (haut de la pile = dernier élément)
    val discard: MutableList<Card>,      // poubelle (dernier élément = visible)
    val humanBoard: PlayerBoard,
    val aiBoard: PlayerBoard,
    var currentPlayer: PlayerId,
    var roundOver: Boolean = false
)

class CabotGame(startingPlayer: PlayerId = PlayerId.HUMAN) {
    val state: CabotGameState

    init {
        val deck = createShuffledDeck()
        val humanBoard = PlayerBoard()
        val aiBoard = PlayerBoard()

        dealInitialBoards(deck, humanBoard, aiBoard)

        state = CabotGameState(
            deck = deck,
            discard = mutableListOf(),
            humanBoard = humanBoard,
            aiBoard = aiBoard,
            currentPlayer = startingPlayer
        )
    }

    companion object {

        // Crée le paquet complet (2 à As pour 4 couleurs + 2 jokers) et le mélange
        fun createShuffledDeck(): MutableList<Card> {
            val cards = mutableListOf<Card>()
            var nextId = 0

            val normalSuits = listOf(Suit.HEARTS, Suit.DIAMONDS, Suit.CLUBS, Suit.SPADES)
            val ranksWithoutJoker = listOf(
                Rank.TWO, Rank.THREE, Rank.FOUR, Rank.FIVE, Rank.SIX,
                Rank.SEVEN, Rank.EIGHT, Rank.NINE, Rank.TEN,
                Rank.JACK, Rank.QUEEN, Rank.KING, Rank.ACE
            )

            // 52 cartes "classiques"
            for (suit in normalSuits) {
                for (rank in ranksWithoutJoker) {
                    cards += Card(
                        suit = suit,
                        rank = rank,
                        id = nextId++
                    )
                }
            }

            // 2 jokers
            repeat(2) {
                cards += Card(
                    suit = Suit.JOKER,
                    rank = Rank.JOKER,
                    id = nextId++
                )
            }

            cards.shuffle(Random(System.currentTimeMillis()))
            return cards
        }

        // Distribue 4 cartes à chaque joueur depuis le dessus de la pioche
        private fun dealInitialBoards(
            deck: MutableList<Card>,
            humanBoard: PlayerBoard,
            aiBoard: PlayerBoard
        ) {
            repeat(4) { index ->
                humanBoard.slots[index] = deck.popLast()
                aiBoard.slots[index] = deck.popLast()
            }
        }
    }

    // Pioche la carte du dessus du deck, ou null si vide
    fun drawFromDeck(): Card? {
        if (state.deck.isEmpty()) return null
        return state.deck.popLast()
    }

    // Pioche la dernière carte de la poubelle (visible), ou null si vide
    fun drawFromDiscard(): Card? {
        if (state.discard.isEmpty()) return null
        return state.discard.popLast()
    }

    // Défausser une carte vers la poubelle
    fun discard(card: Card) {
        state.discard += card
    }

    // Remplacer une carte dans le board d'un joueur par une nouvelle, et défausser l'ancienne
    fun replaceCardOnBoard(playerId: PlayerId, slotIndex: Int, newCard: Card): Boolean {
        if (slotIndex !in 0..3) return false

        val board = if (playerId == PlayerId.HUMAN) state.humanBoard else state.aiBoard
        val oldCard = board.slots[slotIndex] ?: return false

        board.slots[slotIndex] = newCard
        state.discard += oldCard
        return true
    }

    // Calculer les points actuels d'un joueur
    fun getPlayerPoints(playerId: PlayerId): Int {
        val board = if (playerId == PlayerId.HUMAN) state.humanBoard else state.aiBoard
        return board.totalPoints()
    }

    // Changer de joueur
    fun endTurn() {
        state.currentPlayer =
            if (state.currentPlayer == PlayerId.HUMAN) PlayerId.AI else PlayerId.HUMAN
    }

    // Tour très simple de l'IA : pioche, puis remplace ou défausse
    fun playBasicAiTurn() {
        val drawn: Card? = if (state.deck.isNotEmpty()) {
            drawFromDeck()
        } else {
            drawFromDiscard()
        }

        if (drawn == null) return

        val board = state.aiBoard.slots
        val nonEmptyIndices = board.indices.filter { board[it] != null }

        if (nonEmptyIndices.isEmpty()) {
            discard(drawn)
            return
        }

        val indexOfWorst = nonEmptyIndices.maxBy { idx -> board[idx]!!.points() }
        val worstCard = board[indexOfWorst]!!

        if (drawn.points() < worstCard.points()) {
            replaceCardOnBoard(PlayerId.AI, indexOfWorst, drawn)
        } else {
            discard(drawn)
        }
    }
}
