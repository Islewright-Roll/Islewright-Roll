package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

import com.ridgelineapps.resdicegame.Tutorial.Snapshot;
import com.ridgelineapps.resdicegame.Tutorial.Step;

/**
 * The tutorial's decisions: which callout is due in which state, and when a step is done.
 * A small Table stands in for the game, one field per thing the snapshot reports, so each
 * test reads as a few moves of play.
 */
public class TutorialTest {

   /** The game as the tutorial sees it, with the moves a player makes. */
   private static final class Table {
      final Tutorial tutorial;
      int rolls;
      int turns;
      boolean held;
      boolean offered;
      boolean built;
      boolean undo;
      boolean endsTurn;
      int gold;
      int mini;
      boolean classic;
      boolean over;

      Table(Tutorial tutorial) {
         this.tutorial = tutorial;
      }

      Step look() {
         Snapshot s = new Snapshot();
         s.rolls = rolls;
         s.turnsTaken = turns;
         s.anyHeld = held;
         s.offered = offered;
         s.builtThisTurn = built;
         s.canUndo = undo;
         s.rollEndsTurn = endsTurn && !over;
         s.goldDice = gold;
         s.miniDice = mini;
         s.classicBank = classic;
         s.gameOver = over;
         return tutorial.update(s);
      }

      Step roll() {
         rolls++;
         endsTurn = rolls >= 3 || built;
         return look();
      }

      Step hold() {
         held = true;
         return look();
      }

      Step build() {
         built = true;
         undo = true;
         endsTurn = true;
         offered = false;
         return look();
      }

      Step takeBack() {
         built = false;
         undo = false;
         endsTurn = rolls >= 3;
         return look();
      }

      /** The roll button when it ends the turn. */
      Step endTurn() {
         turns++;
         rolls = 0;
         held = false;
         built = false;
         undo = false;
         endsTurn = false;
         offered = false;
         gold = 0;
         return look();
      }
   }

   private static Table fresh() {
      return new Table(new Tutorial(true, 0));
   }

   // ---- the order of 1 to 7 ----------------------------------------------------------

   @Test
   public void aNewGameStartsWithTheRollButton() {
      assertEquals(Step.ROLL, fresh().look());
   }

   @Test
   public void theFirstSevenStepsComeInOrder() {
      Table t = fresh();
      assertEquals(Step.ROLL, t.look());
      assertEquals(Step.KEEP, t.roll());
      assertEquals(Step.COSTS, t.hold());
      t.offered = true;
      assertEquals(Step.COSTS, t.look());
      t.tutorial.gotIt();
      assertEquals(Step.BUILD, t.look());
      assertEquals(Step.UNDO, t.build());
      t.tutorial.gotIt();
      assertEquals(Step.END, t.look());
      // The second turn: the ship
      assertEquals(Step.SHIP, t.endTurn());
      t.tutorial.gotIt();
      assertNull(t.look());
      assertEquals(0x7F, t.tutorial.seenMask());
      assertTrue(t.tutorial.isActive());
   }

   @Test
   public void rollingTheDiceEndsTheRollCallout() {
      Table t = fresh();
      assertEquals(Step.ROLL, t.look());
      assertFalse(t.tutorial.isSeen(Step.ROLL));
      t.roll();
      assertTrue(t.tutorial.isSeen(Step.ROLL));
   }

   @Test
   public void keepShowsAfterTheFirstRollAndEndsOnAHold() {
      Table t = fresh();
      t.look();
      assertEquals(Step.KEEP, t.roll());
      assertFalse(t.tutorial.isSeen(Step.KEEP));
      assertEquals(Step.COSTS, t.hold());
      assertTrue(t.tutorial.isSeen(Step.KEEP));
   }

   @Test
   public void keepEndsOnTheSecondRollToo() {
      Table t = fresh();
      t.look();
      assertEquals(Step.KEEP, t.roll());
      assertEquals(Step.COSTS, t.roll());
      assertTrue(t.tutorial.isSeen(Step.KEEP));
   }

   @Test
   public void keepEndsWhenTheFirstRollIsFollowedByABuild() {
      Table t = fresh();
      t.look();
      t.roll();
      t.offered = true;
      t.build();
      assertTrue(t.tutorial.isSeen(Step.KEEP));
      assertTrue(t.tutorial.isSeen(Step.BUILD));
   }

   @Test
   public void costsEndsOnTheNextRoll() {
      Table t = fresh();
      t.look();
      t.roll();
      assertEquals(Step.COSTS, t.hold());
      assertNull(t.roll());
      assertTrue(t.tutorial.isSeen(Step.COSTS));
   }

   @Test
   public void costsEndsOnGotIt() {
      Table t = fresh();
      t.look();
      t.roll();
      assertEquals(Step.COSTS, t.hold());
      t.tutorial.gotIt();
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.COSTS));
   }

   @Test
   public void costsEndsOnABuildAndTheUndoCalloutFollows() {
      Table t = fresh();
      t.look();
      t.roll();
      t.hold();
      t.offered = true;
      assertEquals(Step.COSTS, t.look());
      assertEquals(Step.UNDO, t.build());
      assertTrue(t.tutorial.isSeen(Step.COSTS));
      assertTrue(t.tutorial.isSeen(Step.BUILD));
   }

   @Test
   public void buildWaitsForSomethingToBeOnOffer() {
      Table t = fresh();
      t.look();
      t.roll();
      t.hold();
      t.tutorial.gotIt();
      assertNull(t.look());
      t.offered = true;
      assertEquals(Step.BUILD, t.look());
      // The offer goes (the dice changed): the callout goes with it but is not used up
      t.offered = false;
      assertNull(t.look());
      assertFalse(t.tutorial.isSeen(Step.BUILD));
      t.offered = true;
      assertEquals(Step.BUILD, t.look());
   }

   @Test
   public void buildNeedsCostsToBeDoneFirst() {
      Table t = fresh();
      t.look();
      t.roll();
      t.hold();
      t.offered = true;
      assertEquals(Step.COSTS, t.look());
   }

   @Test
   public void undoShowsRightAfterTheFirstBuildAndEndsOnGotIt() {
      Table t = fresh();
      reachBuild(t);
      assertEquals(Step.UNDO, t.build());
      t.tutorial.gotIt();
      assertTrue(t.tutorial.isSeen(Step.UNDO));
      assertEquals(Step.END, t.look());
   }

   @Test
   public void undoEndsWhenTheBuildIsTakenBack() {
      Table t = fresh();
      reachBuild(t);
      assertEquals(Step.UNDO, t.build());
      t.offered = true;
      // Nothing follows: the build was done, the undo was done
      assertNull(t.takeBack());
      assertTrue(t.tutorial.isSeen(Step.UNDO));
      assertTrue(t.tutorial.isSeen(Step.BUILD));
   }

   @Test
   public void undoEndsWhenTheTurnEndsWithoutAGotIt() {
      Table t = fresh();
      reachBuild(t);
      assertEquals(Step.UNDO, t.build());
      t.endTurn();
      assertTrue(t.tutorial.isSeen(Step.UNDO));
      // The player used the end turn button, so there is nothing left to say about it
      assertTrue(t.tutorial.isSeen(Step.END));
      assertEquals(Step.SHIP, t.look());
   }

   @Test
   public void endShowsWhenTheRollsAreUsedUpWithNothingToBuild() {
      Table t = fresh();
      t.look();
      t.roll();
      t.roll();
      t.tutorial.gotIt();
      assertEquals(Step.END, t.roll());
      assertEquals(Step.SHIP, t.endTurn());
      assertTrue(t.tutorial.isSeen(Step.END));
   }

   @Test
   public void endWaitsBehindABuildCalloutWhenSomethingIsOnOffer() {
      Table t = fresh();
      t.look();
      t.roll();
      t.roll();
      t.tutorial.gotIt();
      t.offered = true;
      t.roll();
      assertEquals(Step.BUILD, t.look());
   }

   @Test
   public void endShowsAfterABuildOnceUndoIsDone() {
      Table t = fresh();
      reachBuild(t);
      assertEquals(Step.UNDO, t.build());
      t.tutorial.gotIt();
      assertEquals(Step.END, t.look());
      assertEquals(Step.SHIP, t.endTurn());
      assertTrue(t.tutorial.isSeen(Step.END));
   }

   @Test
   public void shipShowsFromTheSecondTurnAndEndsWithGotItOrTheTurn() {
      Table t = fresh();
      t.look();
      t.roll();
      t.endTurn();
      // Turn one ended after a single roll: nothing was built, nothing held
      assertTrue(t.tutorial.isSeen(Step.KEEP));
      assertEquals(Step.COSTS, t.look());
      t.tutorial.gotIt();
      assertEquals(Step.SHIP, t.look());
      t.tutorial.gotIt();
      assertNull(t.look());

      Table u = fresh();
      reachBuild(u);
      u.build();
      u.tutorial.gotIt();
      assertEquals(Step.END, u.look());
      assertEquals(Step.SHIP, u.endTurn());
      // Rolling does not take it away; the turn ending does
      assertEquals(Step.SHIP, u.roll());
      assertEquals(Step.SHIP, u.roll());
      assertEquals(Step.SHIP, u.roll());
      assertNull(u.endTurn());
      assertTrue(u.tutorial.isSeen(Step.SHIP));
   }

   // ---- gold and the bank wait for their moment -------------------------------------------

   @Test
   public void goldNeedsTwoGoldDiceAndWaitsBehindAnotherCallout() {
      Table t = fresh();
      t.look();
      t.gold = 1;
      assertEquals(Step.ROLL, t.look());
      t.gold = 2;
      // The roll callout is up: gold waits
      assertEquals(Step.ROLL, t.look());
      t.tutorial.gotIt();
      assertEquals(Step.GOLD, t.look());
   }

   @Test
   public void aSingleGoldShowsNothing() {
      Table t = new Table(new Tutorial(true, 0x7F));
      t.turns = 3;
      t.rolls = 1;
      t.gold = 1;
      t.look();
      assertNull(t.look());
   }

   @Test
   public void goldIsDoneWhenTheGoldIsGone() {
      Table t = new Table(new Tutorial(true, 0x7F));
      t.turns = 3;
      t.rolls = 1;
      t.look();
      t.gold = 2;
      assertEquals(Step.GOLD, t.look());
      t.gold = 0;
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.GOLD));
      t.gold = 2;
      assertNull(t.look());
   }

   @Test
   public void goldDoesNotBumpACalloutThatIsUp() {
      Table t = fresh();
      t.look();
      t.roll();
      t.gold = 2;
      assertEquals(Step.KEEP, t.look());
      assertEquals(Step.COSTS, t.hold());
      t.tutorial.gotIt();
      assertEquals(Step.GOLD, t.look());
   }

   @Test
   public void bankShowsWhileAMiniDieIsWaitingAndEndsOnGotIt() {
      Table t = new Table(new Tutorial(true, 0x7F));
      t.turns = 4;
      t.look();
      t.mini = 1;
      assertEquals(Step.BANK, t.look());
      assertEquals(Step.BANK, t.roll());
      t.tutorial.gotIt();
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.BANK));
   }

   @Test
   public void bankEndsWhenTheMiniDieIsSpent() {
      Table t = new Table(new Tutorial(true, 0x7F));
      t.turns = 4;
      t.look();
      t.mini = 1;
      assertEquals(Step.BANK, t.look());
      t.mini = 0;
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.BANK));
   }

   @Test
   public void goldComesBeforeBankAndEachWaitsForTheOneUp() {
      Table t = new Table(new Tutorial(true, 0));
      t.look();
      t.gold = 2;
      t.mini = 1;
      assertEquals(Step.ROLL, t.look());
      t.tutorial.gotIt();
      assertEquals(Step.GOLD, t.look());
      t.tutorial.gotIt();
      assertEquals(Step.BANK, t.look());
      t.tutorial.gotIt();
      assertNull(t.look());
   }

   @Test
   public void classicModeShowsTheDragCalloutInsteadAndItEndsWhenTheDieIsDragged() {
      Table t = new Table(new Tutorial(true, 0x7F));
      t.turns = 4;
      t.classic = true;
      t.look();
      t.mini = 1;
      assertEquals(Step.BANK_CLASSIC, t.look());
      t.mini = 0;
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.BANK_CLASSIC));
      assertFalse(t.tutorial.isSeen(Step.BANK));
   }

   @Test
   public void aPlayerWhoLearnedTheUsualBankStillMeetsTheDragOnceClassicIsOn() {
      Table t = new Table(new Tutorial(true, 0x7F | Step.GOLD.bit()));
      t.turns = 4;
      t.look();
      t.mini = 1;
      assertEquals(Step.BANK, t.look());
      t.tutorial.gotIt();
      assertNull(t.look());
      // The tour is over without waiting for the drag, which is a standing hint
      assertFalse(t.tutorial.isActive());
      t.classic = true;
      assertEquals(Step.BANK_CLASSIC, t.look());
      t.tutorial.gotIt();
      assertNull(t.look());
      assertEquals(Tutorial.KNOWN, t.tutorial.seenMask());
   }

   @Test
   public void skippingTheTourLeavesTheDragHintForWhenClassicModeBanksAResource() {
      Table t = new Table(new Tutorial(true, 0x7F));
      t.turns = 4;
      t.classic = true;
      t.look();
      t.tutorial.skip();
      assertFalse(t.tutorial.isActive());
      t.mini = 1;
      assertEquals(Step.BANK_CLASSIC, t.look());
      // Skip with the hint up dismisses it for good
      t.tutorial.skip();
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.BANK_CLASSIC));
   }

   @Test
   public void aPlayerWhoFinishedTheTourLongAgoMeetsTheDragHintOnce() {
      Table t = new Table(new Tutorial(false, Tutorial.ALL));
      t.turns = 9;
      t.mini = 1;
      assertNull(t.look());
      t.classic = true;
      assertEquals(Step.BANK_CLASSIC, t.look());
      assertFalse(t.tutorial.takeDirty());
      t.mini = 0;
      assertNull(t.look());
      assertTrue(t.tutorial.isSeen(Step.BANK_CLASSIC));
      assertTrue(t.tutorial.takeDirty());
      t.mini = 1;
      assertNull(t.look());
   }

   // ---- skip, finish, game over --------------------------------------------------------------

   @Test
   public void skipEndsEverythingForGood() {
      Table t = fresh();
      assertEquals(Step.ROLL, t.look());
      t.tutorial.skip();
      assertFalse(t.tutorial.isActive());
      assertTrue(t.tutorial.takeDirty());
      assertNull(t.look());
      assertNull(t.roll());
      t.gold = 2;
      t.mini = 2;
      assertNull(t.look());
   }

   @Test
   public void skipWorksWithNoCalloutUp() {
      Tutorial tutorial = new Tutorial(true, 0);
      tutorial.skip();
      assertFalse(tutorial.isActive());
   }

   @Test
   public void aTutorialThatIsOffShowsNothing() {
      Table t = new Table(new Tutorial(false, 0));
      assertNull(t.look());
      t.gold = 2;
      assertNull(t.look());
   }

   @Test
   public void theTutorialIsOverWhenEveryTourStepIsSeen() {
      Tutorial tutorial = new Tutorial(true, Tutorial.ALL & ~Step.BANK.bit());
      Table t = new Table(tutorial);
      t.turns = 5;
      assertNull(t.look());
      assertTrue(tutorial.isActive());
      t.mini = 1;
      assertEquals(Step.BANK, t.look());
      tutorial.gotIt();
      assertNull(t.look());
      assertFalse(tutorial.isActive());
      assertEquals(Tutorial.ALL, tutorial.seenMask());
      assertTrue(tutorial.takeDirty());
   }

   @Test
   public void nothingShowsOnceTheGameIsOver() {
      Table t = fresh();
      assertEquals(Step.ROLL, t.look());
      t.over = true;
      assertNull(t.look());
      t.gold = 2;
      t.mini = 3;
      assertNull(t.look());
      assertNull(t.tutorial.current());
   }

   @Test
   public void aNewGameAfterTheEndGoesOnWhereItLeftOff() {
      Table t = fresh();
      reachBuild(t);
      t.build();
      t.tutorial.gotIt();
      t.look();
      t.endTurn();
      t.tutorial.gotIt();
      assertEquals(0x7F, t.tutorial.seenMask());
      t.turns = 15;
      t.over = true;
      assertNull(t.look());
      // Play again: the turn count is back to zero and the rolls too
      t.turns = 0;
      t.rolls = 0;
      t.over = false;
      t.built = false;
      t.undo = false;
      t.endsTurn = false;
      t.held = false;
      assertNull(t.look());
      assertEquals(0x7F, t.tutorial.seenMask());
   }

   @Test
   public void aRestartMidTutorialShowsTheNextPendingStepAfterIt() {
      Table t = fresh();
      t.look();
      t.roll();
      assertEquals(Step.KEEP, t.look());
      // Restart from the menu: back to turn one, no rolls. ROLL is done, KEEP is not.
      t.rolls = 0;
      t.held = false;
      t.endsTurn = false;
      assertNull(t.look());
      assertEquals(Step.KEEP, t.roll());
   }

   // ---- resuming ----------------------------------------------------------------------------

   @Test
   public void aSavedMaskResumesWhereItLeftOff() {
      // ROLL, KEEP, COSTS and BUILD seen; the game is at the start of turn two
      Tutorial tutorial = new Tutorial(true, Step.ROLL.bit() | Step.KEEP.bit() | Step.COSTS.bit() | Step.BUILD.bit());
      Table t = new Table(tutorial);
      t.turns = 1;
      assertEquals(Step.SHIP, t.look());
   }

   @Test
   public void aFreshInstallReopenedBeforeTheFirstRollShowsTheRollCalloutAgain() {
      Table first = fresh();
      assertEquals(Step.ROLL, first.look());
      Table reopened = new Table(new Tutorial(true, first.tutorial.seenMask()));
      assertEquals(Step.ROLL, reopened.look());
   }

   @Test
   public void reopenedAfterTheFirstRollTheKeepCalloutIsStillDue() {
      Table first = fresh();
      first.look();
      first.roll();
      assertEquals(Step.ROLL.bit(), first.tutorial.seenMask());
      Table reopened = new Table(new Tutorial(true, first.tutorial.seenMask()));
      reopened.rolls = 1;
      assertEquals(Step.KEEP, reopened.look());
   }

   @Test
   public void reopenedWithAnUnseenTutorialInTheMiddleOfAGameCatchesUp() {
      // An upgrade with a saved game on turn five, three rolls used: the early steps are moot
      Table t = fresh();
      t.turns = 4;
      t.rolls = 3;
      t.endsTurn = true;
      assertEquals(Step.COSTS, t.look());
      assertTrue(t.tutorial.isSeen(Step.ROLL));
      assertTrue(t.tutorial.isSeen(Step.KEEP));
      t.tutorial.gotIt();
      assertEquals(Step.END, t.look());
   }

   @Test
   public void configureTakesNewSettingsAndRestartsTheWatching() {
      Tutorial tutorial = new Tutorial(true, 0x1F);
      tutorial.takeDirty();
      tutorial.configure(true, 0x1F);
      assertFalse(tutorial.takeDirty());
      tutorial.configure(false, 0x1F);
      assertFalse(tutorial.isActive());
      tutorial.configure(true, 0);
      assertTrue(tutorial.isActive());
      assertEquals(0, tutorial.seenMask());
      Table t = new Table(tutorial);
      assertEquals(Step.ROLL, t.look());
   }

   @Test
   public void progressIsReportedDirtyOnlyWhenItChanges() {
      Table t = fresh();
      t.look();
      assertFalse(t.tutorial.takeDirty());
      t.roll();
      assertTrue(t.tutorial.takeDirty());
      assertFalse(t.tutorial.takeDirty());
      t.look();
      assertFalse(t.tutorial.takeDirty());
   }

   @Test
   public void aSeenMaskKeepsOnlyTheBitsForRealSteps() {
      assertEquals(Tutorial.KNOWN, new Tutorial(true, -1).seenMask());
      assertEquals(10, Step.values().length);
   }

   // ---- the events can be spoken directly ----------------------------------------------------

   @Test
   public void theEventMethodsMarkTheirStepsDone() {
      Tutorial tutorial = new Tutorial(true, 0);
      tutorial.rolled(1);
      assertEquals(Step.ROLL.bit(), tutorial.seenMask());
      tutorial.held();
      assertEquals(Step.ROLL.bit() | Step.KEEP.bit(), tutorial.seenMask());
      tutorial.built();
      assertTrue(tutorial.isSeen(Step.BUILD));
      // The ones that need their callout up first do nothing without it
      tutorial.undone();
      assertFalse(tutorial.isSeen(Step.UNDO));
      tutorial.turnEnded();
      assertTrue(tutorial.isSeen(Step.END));
      assertFalse(tutorial.isSeen(Step.COSTS));
   }

   private static void reachBuild(Table t) {
      t.look();
      t.roll();
      t.hold();
      t.tutorial.gotIt();
      t.offered = true;
      assertEquals(Step.BUILD, t.look());
   }
}
